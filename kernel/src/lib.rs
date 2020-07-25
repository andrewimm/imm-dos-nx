#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(llvm_asm)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]

#![no_std]

// Test-safe modules
pub mod buffers;
pub mod files;
pub mod memory;
pub mod pipes;
pub mod promise;
pub mod time;

#[cfg(not(test))]
pub mod debug;
#[cfg(not(test))]
pub mod devices;
#[cfg(not(test))]
pub mod disks;
#[cfg(not(test))]
pub mod drivers;
#[cfg(not(test))]
pub mod filesystems;
#[cfg(not(test))]
pub mod gdt;
#[cfg(not(test))]
pub mod hardware;
#[cfg(not(test))]
pub mod idt;
#[cfg(not(test))]
pub mod init;
#[cfg(not(test))]
pub mod input;
#[cfg(not(test))]
pub mod interrupts;
#[cfg(not(test))]
pub mod panic;
#[cfg(not(test))]
pub mod process;
#[cfg(not(test))]
pub mod syscalls;
#[cfg(not(test))]
pub mod tty;
#[cfg(not(test))]
pub mod x86;

use memory::address::PhysicalAddress;

extern crate alloc;

#[cfg(not(test))]
extern {
  // Segment labels from the linker script
  // Makes it easy to mark pages as readable / writable
  #[link_name = "__ro_physical_start"]
  static label_ro_physical_start: u8;
  #[link_name = "__ro_physical_end"]
  static label_ro_physical_end: u8;
  #[link_name = "__rw_physical_start"]
  static label_rw_physical_start: u8;
  #[link_name = "__rw_physical_end"]
  static label_rw_physical_end: u8;
  #[link_name = "__bss_start"]
  static label_bss_start: u8;
  #[link_name = "__bss_end"]
  static label_bss_end: u8;
  #[link_name = "__stack_start"]
  static label_stack_start: u8;
}

// Used to pass data from the bootloader to the kernel
#[repr(C, packed)]
pub struct BootStruct {
  initfs_start: usize,
  initfs_size: usize,
}

/**
 * Clear the .bss section. Since we copied bytes from disk to memory, there's a
 * chance it contains the symbol table.
 */
#[cfg(not(test))]
unsafe fn zero_bss() {
  let mut bss_iter = &label_bss_start as *const u8 as usize;
  let bss_end = &label_bss_end as *const u8 as usize;
  while bss_iter < bss_end {
    let bss_ptr = bss_iter as *mut u8;
    *bss_ptr = 0;
    bss_iter += 1;
  }
}

#[cfg(not(test))]
unsafe fn init_tables() {
  idt::init();
  gdt::init();
}

#[cfg(not(test))]
unsafe fn init_memory_new() {
  let allocator_location = &label_rw_physical_end as *const u8 as usize;
  memory::physical::init_allocator(allocator_location, 0x1000);

  let stack_start_address = PhysicalAddress::new(&label_stack_start as *const u8 as usize);
  let kernel_data_bounds = memory::virt::KernelDataBounds {
    ro_start: PhysicalAddress::new(&label_ro_physical_start as *const u8 as usize),
    rw_end: PhysicalAddress::new(&label_rw_physical_end as *const u8 as usize),
    stack_start: stack_start_address,
  };

  let initial_pagedir = memory::virt::create_initial_pagedir();
  memory::virt::map_kernel(initial_pagedir, &kernel_data_bounds);
  initial_pagedir.make_active();
  memory::virt::enable_paging();

  memory::physical::move_allocator_reference_to_highmem();

  // move esp to the higher page, maintaining its relative location in the frame
  unsafe {
    llvm_asm!(
      "mov eax, esp
      sub eax, $0
      add eax, 0xffbfe000
      mov esp, eax" : :
      "r"(stack_start_address.as_u32()) :
      "eax", "esp" :
      "intel", "volatile"
    );
  }

  memory::high_jump();
  // CLOWNTOWN: ebx points to the PLT, and is initialized based on a relative
  // position to the starting instruction pointer. Since we start in lowmem and
  // then jump to highmem, it gets set to a location in user memory space, which
  // breaks everything the first time a user process unmaps the low copy of the
  // kernel.
  // If we had a bootloader that initialized a page table and mapped us into
  // highmem before entering the kernel, this wouldn't be necessary...
  llvm_asm!("or ebx, 0xc0000000" : : : : "intel", "volatile");

  kprintln!("\nKernel range: {:?}-{:?}", kernel_data_bounds.ro_start, kernel_data_bounds.rw_end);
}

/**
 * Entry point of the kernel
 */
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start(boot_struct_ptr: *const BootStruct) -> ! {
  let initfs_start = unsafe {
    let boot_struct = &*boot_struct_ptr;
    boot_struct.initfs_start
  } | 0xc0000000;

  unsafe {
    let boot_struct = &*boot_struct_ptr;
    zero_bss();
    init_memory_new();
    init_tables();
  }

  unsafe {
    kprintln!("\nEntering the Kernel...");

    kprintln!(
      "\nTotal Memory: {} KiB\nFree Memory: {} KiB",
      memory::physical::get_frame_count() * 4,
      memory::physical::get_free_frame_count() * 4,
    );

    let heap_start = memory::address::VirtualAddress::new(0xc0400000);
    {
      let heap_size_frames = memory::heap::INITIAL_HEAP_SIZE;
      memory::heap::map_allocator(heap_start, heap_size_frames);
      let heap_size = heap_size_frames * 0x1000;
      kprintln!("Kernel heap at {:?}-{:?}", heap_start, memory::address::VirtualAddress::new(0xc0400000 + heap_size));
      memory::heap::init_allocator(heap_start, heap_size);
    }
    memory::physical::init_refcount();

    // Initialize hardware
    devices::init();
    tty::init_ttys();
    time::system::initialize_from_rtc();

    filesystems::init_fs();

    let init_fs = filesystems::init::InitFileSystem::new(memory::address::VirtualAddress::new(initfs_start));
    let boxed_fs = alloc::boxed::Box::new(init_fs);
    filesystems::VFS.register_fs("INIT", boxed_fs).expect("Failed to register INIT FS");

    process::init();
    let init_process = process::all_processes_mut().spawn_first_process(heap_start);
    process::make_current(init_process);
  }

  let current_time = time::system::get_system_time().to_timestamp().to_datetime();
  tty::console_write(format_args!("System Time: {:} {:}\n", current_time.date, current_time.time));

  // Test pipes
  {
    let (read, write) = pipes::PIPES.create().unwrap();
    let pipe_test = "TEST THE PIPE";
    pipes::PIPES.write(write, pipe_test.as_bytes());
    let mut buffer: [u8; 13] = [0; 13];
    let bytes_read = pipes::PIPES.read(read, &mut buffer).unwrap();
    assert_eq!(bytes_read, 13);
    assert_eq!(pipe_test.as_bytes(), &buffer);
  }

  {
    let input_proc = process::all_processes_mut().fork_current();
    process::set_kernel_mode_function(input_proc, input::run_input);

    let disk_proc = process::all_processes_mut().fork_current();
    process::set_kernel_mode_function(disk_proc, disks::init);

    let ttys_proc = process::all_processes_mut().fork_current();
    process::set_kernel_mode_function(ttys_proc, tty::ttys_process);
  }

  // Spawn init process
  let init_proc_id = process::all_processes_mut().fork_current();
  {
    let mut processes = process::all_processes_mut();
    let init_proc = processes.get_process(init_proc_id).unwrap();
    init_proc.set_initial_entry_point(user_init, 0xbffffffc);
  }

  process::enter_usermode(init_proc_id);

  loop {
    unsafe {
      llvm_asm!("cli" : : : : "volatile");
      process::yield_coop();
      llvm_asm!("sti; hlt" : : : : "volatile");
    }
  }
}

#[inline(never)]
pub extern fn user_init() {
  let com1 = syscall::open("DEV:\\COM1");
  let intro = "Opened COM1. ";
  syscall::write(com1, intro.as_ptr(), intro.len());

  let start = "Forking child. ";
  syscall::write(com1, start.as_ptr(), start.len());
  let pid = syscall::fork();
  if pid == 0 {
    let ch_start = "Child start. ";
    syscall::write(com1, ch_start.as_ptr(), ch_start.len());
    syscall::sleep(1000);
    let ch_end = "Child end. ";
    syscall::write(com1, ch_end.as_ptr(), ch_end.len());
    syscall::exit(0x10);
  } else {
    let (_, code) = syscall::wait_pid(pid);
    let back = "Back to parent. ";
    syscall::write(com1, back.as_ptr(), back.len());
    loop {
      syscall::yield_coop();
    }
  }

  /*
  let pid = syscall::fork();
  let ticktock = if pid == 0 {
    "TOCK "
  } else {
    "TICK "
  };
  loop {
    syscall::write(com1, ticktock.as_ptr(), ticktock.len());
    syscall::sleep(1000);
  }
  */

  /*
  let pid = syscall::fork();
  if pid == 0 {
    let tty0 = syscall::open("DEV:\\TTY0");
    let prompt = "\nA:\\>";
    syscall::write(tty0, prompt.as_ptr(), prompt.len());

    loop {
      syscall::yield_coop();
    }
  } else {
    let msg = "TICK";
    loop {
      syscall::write(com1, msg.as_ptr(), msg.len());
      syscall::sleep(1000);
    }
  }
  */
}