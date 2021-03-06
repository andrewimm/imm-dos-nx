use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use crate::devices;
use crate::files::{handle::{Handle, HandleAllocator, LocalHandle}, cursor::SeekMethod};
use spin::RwLock;
use super::filesystem::FileSystem;
use syscall::files::DirEntryInfo;

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
  fn open(&self, path: &str) -> Result<LocalHandle, ()> {
    let local_path = if path.starts_with('\\') {
      &path[1..]
    } else {
      path
    };

    // temporary, switch device registration to use strings too
    let mut name: [u8; 8] = [0x20; 8];
    {
      let mut i = 0;
      let bytes = local_path.as_bytes();
      while i < 8 && i < bytes.len() {
        name[i] = bytes[i];
        i += 1;
      }
    }
  
    // needs to account for directories
    match devices::get_device_number_by_name(&name) {
      Some(number) => {
        let handle = self.handle_allocator.get_next();
        {
          let driver = devices::get_driver_for_device(number).ok_or(())?;
          driver.open(handle)?;
        }
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

  fn dup(&self, handle: LocalHandle) -> Result<LocalHandle, ()> {
    let device = self.get_device_for_handle(handle);
    match device {
      Some(number) => {
        let new_handle = self.handle_allocator.get_next();
        let mut handle_to_device = self.handle_to_device.write();
        handle_to_device.push(Some(number));
        Ok(new_handle)
      },
      None => Err(()),
    }
  }

  fn ioctl(&self, handle: LocalHandle, command: u32, _arg: u32) -> Result<u32, ()> {
    match command {
      0 => { // Identify device number
        self.get_device_for_handle(handle).map(|d| d as u32).ok_or(())
      },
      _ => Err(())
    }
  }

  fn seek(&self, handle: LocalHandle, offset: SeekMethod) -> Result<usize, ()> {
    match self.get_device_for_handle(handle) {
      Some(number) => {
        let driver = devices::get_driver_for_device(number).ok_or(())?;
        match driver.seek(handle, offset) {
          Ok(position) => Ok(position),
          Err(_) => Err(())
        }
      },
      None => Err(())
    }
  }

  fn open_dir(&self, path: &str) -> Result<LocalHandle, ()> {
    Err(())
  }

  fn read_dir(&self, handle: LocalHandle, index: usize, info: &mut DirEntryInfo) -> Result<(), ()> {
    Err(())
  }
}