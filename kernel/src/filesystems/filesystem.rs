use crate::files::{cursor::SeekMethod, handle::LocalHandle};
use syscall::files::DirEntryInfo;

pub trait FileSystem {
  fn open(&self, path: &str) -> Result<LocalHandle, ()>;
  fn read(&self, handle: LocalHandle, buffer: &mut [u8]) -> Result<usize, ()>;
  fn write(&self, handle: LocalHandle, buffer: &[u8]) -> Result<usize, ()>;
  fn close(&self, handle: LocalHandle) -> Result<(), ()>;
  fn dup(&self, handle: LocalHandle) -> Result<LocalHandle, ()>;
  fn seek(&self, handle: LocalHandle, offset: SeekMethod) -> Result<usize, ()>;
  
  fn open_dir(&self, path: &str) -> Result<LocalHandle, ()>;
  fn read_dir(&self, handle: LocalHandle, index: usize, info: &mut DirEntryInfo) -> Result<(), ()>;

  fn ioctl(&self, handle: LocalHandle, command: u32, arg: u32) -> Result<u32, ()> {
    Err(())
  }
}