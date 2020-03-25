use alloc::vec::Vec;
use crate::devices;
use crate::drivers::driver::DeviceDriver;
use crate::files::filename::Path;
use crate::files::handle::{Handle, HandleAllocator, LocalHandle};
use spin::RwLock;
use super::filesystem::FileSystem;

pub struct DevFileSystem {
  handle_allocator: HandleAllocator<LocalHandle>,
  handle_to_device: RwLock<Vec<Option<usize>>>,
}

impl DevFileSystem {
  pub const fn new() -> DevFileSystem {
    DevFileSystem {
      handle_allocator: HandleAllocator::<LocalHandle>::new(),
      handle_to_device: RwLock::new(Vec::new()),
    }
  }

  pub fn get_device_for_handle(&self, handle: LocalHandle) -> Option<usize> {
    let handle_to_device = self.handle_to_device.read();
    match handle_to_device.get(handle.as_u32() as usize) {
      Some(option) => *option,
      None => None,
    }
  }
}

impl FileSystem for DevFileSystem {
  fn open(&self, path: &Path) -> Result<LocalHandle, ()> {
    let handle = self.handle_allocator.get_next();
    // needs to account for directories
    match devices::get_device_number_by_name(&path.filename) {
      Some(number) => {
        let mut handle_to_device = self.handle_to_device.write();
        while handle_to_device.len() < handle.as_u32() as usize {
          handle_to_device.push(None);
        }
        handle_to_device.push(Some(number));
        Ok(handle)
      },
      None => Err(()),
    }
  }

  fn read(&self, handle: LocalHandle, buffer: &mut [u8]) -> Result<usize, ()> {
    match self.get_device_for_handle(handle) {
      Some(number) => {
        let driver = devices::get_driver_for_device(number).ok_or(())?;
        match driver.read(handle, buffer) {
          Ok(len) => Ok(len),
          Err(_) => Err(())
        }
      },
      None => Err(())
    }
  }

  fn write(&self, handle: LocalHandle, buffer: &[u8]) -> Result<usize, ()> {
    match self.get_device_for_handle(handle) {
      Some(number) => {
        let driver = devices::get_driver_for_device(number).ok_or(())?;
        match driver.write(handle, buffer) {
          Ok(len) => Ok(len),
          Err(_) => Err(())
        }
      },
      None => Err(())
    }
  }

  fn close(&self, handle: LocalHandle) -> Result<(), ()> {
    Err(())
  }
}