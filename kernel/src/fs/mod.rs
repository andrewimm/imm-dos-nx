pub mod drive;
pub mod drivers;
pub mod filesystem;

use alloc::boxed::Box;
use alloc::sync::Arc;
use crate::memory::address::VirtualAddress;
use filesystem::{FileSystemCategory, KernelFileSystem};

pub static DRIVES: drive::DriveMap = drive::DriveMap::new();

pub fn init_system_drives(initfs_location: VirtualAddress) {
  let initfs = drivers::initfs::InitFileSystem::new(initfs_location);
  DRIVES.mount_drive("INIT", FileSystemCategory::KernelSync, Arc::new(Box::new(initfs)));
  let devfs = drivers::devfs::DevFileSystem::new();
  DRIVES.mount_drive("DEV", FileSystemCategory::KernelAsync, Arc::new(Box::new(devfs)));
}
