use crate::files::handle::{DeviceHandlePair, FileHandle, FileHandleMap, LocalHandle};
use crate::memory;
use crate::memory::address::VirtualAddress;
use crate::memory::physical::frame::Frame;
use crate::memory::virt::page_directory;
use crate::memory::virt::page_table::{PageTable, PageTableReference};
use crate::memory::virt::region::{ExpansionDirection, MemoryRegionType, Permissions, VirtualMemoryRegion};
use crate::time;
use spin::RwLock;
use super::id::ProcessID;
use super::memory::MemoryRegions;

#[derive(Copy, Clone)]
pub enum RunState {
  Running,
  Sleeping(usize),
  // Blocked(PromiseResult),
}

pub struct ProcessState {
  pid: ProcessID,
  parent: ProcessID,

  memory_regions: RwLock<MemoryRegions>,

  /*
  kernel_heap_region: RwLock<VirtualMemoryRegion>,
  kernel_stack_region: RwLock<VirtualMemoryRegion>,
  //heap_region: RwLock<VirtualMemoryRegion>,
  stack_region: RwLock<VirtualMemoryRegion>,
  //execution_regions: RwLock<Vec<VirtualMemoryRegion>>,
  */
  page_directory: PageTableReference,

  kernel_esp: RwLock<usize>,

  open_files: RwLock<FileHandleMap>,

  run_state: RwLock<RunState>,
}

impl ProcessState {
  /**
   * Used to generate the init process, which has no parent
   */
  pub fn first(pid: ProcessID, heap_start: VirtualAddress) -> ProcessState {
    ProcessState {
      pid,
      parent: pid,

      memory_regions: RwLock::new(MemoryRegions::initial(heap_start)),

      /*
      kernel_heap_region: RwLock::new(
        VirtualMemoryRegion::new(
          heap_start,
          memory::heap::INITIAL_HEAP_SIZE * 0x1000,
          MemoryRegionType::Anonymous(ExpansionDirection::After),
          Permissions::ReadOnly,
        ),
      ),
      kernel_stack_region: RwLock::new(
        VirtualMemoryRegion::new(
          memory::virt::STACK_START,
          0x1000,
          MemoryRegionType::Anonymous(ExpansionDirection::Before),
          Permissions::ReadOnly,
        ),
      ),

      stack_region: RwLock::new(
        VirtualMemoryRegion::new(
          VirtualAddress::new(0xc0000000 - 0x2000),
          0x2000,
          MemoryRegionType::Anonymous(ExpansionDirection::Before),
          Permissions::CopyOnWrite,
        ),
      ),
      */

      page_directory: PageTableReference::current(),

      kernel_esp: RwLock::new(0),

      open_files: RwLock::new(FileHandleMap::new()),

      run_state: RwLock::new(RunState::Running),
    }
  }

  /**
   * It is not possible to create an orphaned process; each process must be
   * forked from an existing one.
   */
  pub fn fork(&self, pid: ProcessID) -> ProcessState {
    let new_regions = RwLock::new(self.memory_regions.read().fork());
    let new_pagedir = self.fork_page_directory();
    ProcessState {
      pid,
      parent: self.pid,

      memory_regions: new_regions,

      page_directory: new_pagedir,

      kernel_esp: RwLock::new(
        memory::virt::STACK_START.as_usize() + 0x1000 - 4
      ),

      open_files: RwLock::new(FileHandleMap::new()),

      run_state: RwLock::new(RunState::Running),
    }
  }

  /*
  pub fn fork_page_directory(&self) -> PageTableReference {
    let temp_page_address = page_directory::get_temporary_page_address();
    // Create the top page table, which will contain the temp page and
    // kernel stack
    let top_page = memory::physical::allocate_frame().unwrap();
    page_directory::map_frame_to_temporary_page(top_page);
    PageTable::at_address(temp_page_address).zero();
    // Create the new page directory
    let pagedir_frame = memory::physical::allocate_frame().unwrap();
    page_directory::map_frame_to_temporary_page(pagedir_frame);
    let new_pagedir = PageTable::at_address(temp_page_address);
    new_pagedir.zero();
    // Initialize the page directory with its required mappings
    new_pagedir.get_mut(1023).set_address(pagedir_frame.get_address());
    new_pagedir.get_mut(1023).set_present();
    new_pagedir.get_mut(1022).set_address(top_page.get_address());
    new_pagedir.get_mut(1022).set_present();

    // Copy the kernel heap mappings
    let current_page_address = page_directory::get_current_page_address();
    let current_pagedir = PageTable::at_address(current_page_address);
    {
      let heap_region = self.kernel_heap_region.read();
      let start = heap_region.get_starting_address_as_usize();
      let size = heap_region.get_size();
      let mut offset = 0;
      while offset < size {
        let index = (start + offset) >> 22;
        new_pagedir.get_mut(index).set_address(current_pagedir.get(index).get_address());
        new_pagedir.get_mut(index).set_present();
        offset += 0x400000;
      }
    }

    // Duplicate the process memory mapping
    // Right now this just copies the kernel data. This needs to come from a
    // process-stored map in the future.
    new_pagedir.get_mut(0).set_address(current_pagedir.get(0).get_address());
    new_pagedir.get_mut(0).set_present();
    new_pagedir.get_mut(0).set_user_access();
    new_pagedir.get_mut(0x300).set_address(current_pagedir.get(0x300).get_address());
    new_pagedir.get_mut(0x300).set_present();

    // stack needs to be copy-on-write

    PageTableReference::new(pagedir_frame.get_address())
  }
  */

  pub fn make_current_stack_frame_editable(&self) {
    let esp = self.kernel_esp.read().clone();
    let directory_entry = esp >> 22;
    let table_entry = (esp >> 12) & 0x3ff;
    // Map the page table into temp space
    page_directory::map_frame_to_temporary_page(Frame::new(self.page_directory.get_address().as_usize()));
    let temp_page_address = page_directory::get_temporary_page_address();
    let pagedir = PageTable::at_address(temp_page_address);
    let stack_table_address = pagedir.get(directory_entry).get_address().as_usize();
    page_directory::map_frame_to_temporary_page(Frame::new(stack_table_address));
    if !pagedir.get(table_entry).is_present() {
      let stack_frame = memory::physical::allocate_frame().unwrap();
      pagedir.get_mut(table_entry).set_address(stack_frame.get_address());
      pagedir.get_mut(table_entry).set_present();
    }
    let current_stack_frame = pagedir.get(table_entry).get_address().as_usize();
    page_directory::map_frame_to_temporary_page(Frame::new(current_stack_frame));
  }

  pub fn set_initial_entry_point(&self, func: extern fn(), esp: usize) {
    self.make_current_stack_frame_editable();
    let temp_page_address = page_directory::get_temporary_page_address().as_usize();
    let kernel_esp = self.kernel_esp.read().clone();
    let stack_offset = kernel_esp & 0xfff;
    unsafe {
      let stack_ptr = (temp_page_address + stack_offset) as *mut usize;
      // Stack segment
      *stack_ptr.offset(-1) = 0x23;
      // Stack pointer
      *stack_ptr.offset(-2) = esp;
      // eflags
      *stack_ptr.offset(-3) = 0x200; // interrupt enabled
      // Code segment
      *stack_ptr.offset(-4) = 0x1b;
      // Instruction pointer
      *stack_ptr.offset(-5) = func as usize; 
    }
    *self.kernel_esp.write() = kernel_esp - 4 * 5;
  }

  pub fn get_range_containing_address(&self, addr: VirtualAddress) -> Option<VirtualMemoryRegion> {
    self.memory_regions.read().get_range_containing_address(addr)
  }

  pub fn get_id(&self) -> ProcessID {
    self.pid
  }

  pub fn get_page_directory(&self) -> &PageTableReference {
    &self.page_directory
  }

  pub fn get_memory_regions(&self) -> &RwLock<MemoryRegions> {
    &self.memory_regions
  }

  pub fn get_kernel_stack_pointer(&self) -> usize {
    self.kernel_esp.read().clone()
  }

  pub fn get_kernel_stack_container(&self) -> &RwLock<usize> {
    &self.kernel_esp
  }

  pub fn open_file(&self, drive: usize, local: LocalHandle) -> FileHandle {
    let mut files = self.open_files.write();
    files.open_handle(drive, local)
  }

  pub fn close_file(&self, handle: FileHandle) {
    let mut files = self.open_files.write();
    files.close_handle(handle)
  }

  pub fn get_open_file_info(&self, handle: FileHandle) -> Option<DeviceHandlePair> {
    let files = self.open_files.read();
    files.get_drive_and_handle(handle)
  }

  pub fn sleep(&self, ms: usize) {
    let mut run_state = self.run_state.write();
    *run_state = RunState::Sleeping(ms);
  }

  pub fn update_tick(&self) {
    let run_state = self.run_state.read().clone();
    match run_state {
      RunState::Sleeping(duration) => {
        if duration > time::MS_PER_TICK {
          let remaining = duration - time::MS_PER_TICK;
          *self.run_state.write() = RunState::Sleeping(remaining);
          return;
        }
        *self.run_state.write() = RunState::Running;
      },
      _ => (),
    }
  }

  pub fn is_running(&self) -> bool {
    let run_state = self.run_state.read().clone();
    match run_state {
      RunState::Running => true,
      _ => false
    }
  }
}
