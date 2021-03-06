use alloc::sync::Arc;
use crate::files::cursor::SeekMethod;
use crate::files::handle::LocalHandle;
use crate::files::ioctl::FIONREAD;
use crate::filesystems::filesystem::FileSystem;
use super::collection::PipeCollection;
use syscall::files::DirEntryInfo;

pub struct PipeFileSystem {
  collection: Arc<PipeCollection>,
}

impl PipeFileSystem {
  pub fn new(collection: &Arc<PipeCollection>) -> PipeFileSystem {
    PipeFileSystem {
      collection: Arc::clone(collection),
    }
  }
}

impl FileSystem for PipeFileSystem {
  /// Open only works for named pipes, which are not yet implemented
  fn open(&self, _path: &str) -> Result<LocalHandle, ()> {
    Err(())
  }

  fn read(&self, handle: LocalHandle, buffer: &mut [u8]) -> Result<usize, ()> {
    self.collection.read(handle, buffer).map_err(|_| ())
  }

  fn write(&self, handle: LocalHandle, buffer: &[u8]) -> Result<usize, ()> {
    self.collection.write(handle, buffer).map_err(|_| ())
  }

  fn close(&self, _handle: LocalHandle) -> Result<(), ()> {
    Err(())
  }

  fn dup(&self, _handle: LocalHandle) -> Result<LocalHandle, ()> {
    Err(())
  }

  fn ioctl(&self, handle: LocalHandle, command: u32, arg: u32) -> Result<u32, ()> {
    match command {
      FIONREAD => {
        // Get bytes ready to read
        let bytes = self.collection.get_available_bytes(handle).map_err(|_| ())?;
        let out_ptr = arg as *mut u32;
        unsafe {
          *out_ptr = bytes as u32;
        }
        Ok(0)
      },
      _ => Err(()),
    }
  }

  fn seek(&self, _handle: LocalHandle, _offset: SeekMethod) -> Result<usize, ()> {
    Err(())
  }

  fn open_dir(&self, path: &str) -> Result<LocalHandle, ()> {
    Err(())
  }

  fn read_dir(&self, handle: LocalHandle, index: usize, info: &mut DirEntryInfo) -> Result<(), ()> {
    Err(())
  }
}