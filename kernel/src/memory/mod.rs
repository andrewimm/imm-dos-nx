pub mod address;
pub mod bitmap_frame_allocator;
pub mod frame_allocator;
pub mod frame;
pub mod map;
pub mod paging;

use crate::x86;
use frame_allocator::FrameAllocator;
use bitmap_frame_allocator::BitmapFrameAllocator;

pub static mut FRAME_ALLOCATOR: BitmapFrameAllocator = BitmapFrameAllocator::new();

pub fn init(kernel_start: address::PhysicalAddress, kernel_end: address::PhysicalAddress) {
  unsafe {
    let mem_map = map::load_entries_at_address(0x1000);

    FRAME_ALLOCATOR.init(mem_map, kernel_start, kernel_end);
  }
}

pub fn init_paging() {
  let dir_frame = allocate_physical_frame().unwrap();
  unsafe {
    dir_frame.zero_memory();
  }
  let mut dir = paging::directory::PageDirectory {
    physical_location: dir_frame.get_address(),
    virtual_location: address::VirtualAddress::new(dir_frame.get_address().as_usize()),
  };
  dir.set_last_entry_to_self();

  // identity-map the first 4 MiB
  let table_0_frame = allocate_physical_frame().unwrap();
  let mut table = paging::table::PageTable {
    physical_location: table_0_frame.get_address(),
    virtual_location: address::VirtualAddress::new(table_0_frame.get_address().as_usize()),
  };
  dir.set_table_at_entry(table_0_frame.get_address(), 0);
  for i in 0..1024 {
    let frame_inspect = frame::Frame::new(i * 0x1000);
    let is_occupied = unsafe {
      !FRAME_ALLOCATOR.is_free(frame_inspect)
    };
    let mut entry = paging::entry::PageTableEntry::new();
    if is_occupied || i < 256 {
      entry.set_address(frame_inspect.get_address());
      entry.set_present();
    }
    table.set_table_entry(i, entry);
  }

  dir.make_active();
  x86::registers::enable_paging();
}

pub fn count_frames() -> usize {
  unsafe {
    FRAME_ALLOCATOR.count_frames()
  }
}

pub fn count_free_frames() -> usize {
  unsafe {
    FRAME_ALLOCATOR.count_free_frames()
  }
}

pub fn allocate_physical_frame() -> frame_allocator::FrameAllocatorResult {
  unsafe {
    FRAME_ALLOCATOR.allocate()
  }
}