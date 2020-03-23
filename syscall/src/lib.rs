#![feature(asm)]

#![no_std]

pub mod data;

pub use data::*;

pub fn syscall_inner(method: u32, arg0: u32, arg1: u32, arg2: u32) -> u32 {
  let result: u32;
  unsafe {
    asm!("int 0x2b" :
          "={eax}"(result) :
          "{eax}"(method), "{ebx}"(arg0), "{ecx}"(arg1), "{edx}"(arg2) :
          "eax", "ebx", "ecx", "edx" :
          "intel", "volatile"
    );
  }
  result
}

pub fn debug() -> u32 {
  syscall_inner(0xffff, 0, 0, 0)
}

pub fn open(path: &'static str) -> u32 {
  let path_ptr = StringPtr::from_str(path);
  syscall_inner(0x10, &path_ptr as *const StringPtr as u32, 0, 0)
}

pub fn read(handle: u32, buffer: *mut u8, length: usize) -> usize {
  syscall_inner(0x12, handle, buffer as u32, length as u32) as usize
}