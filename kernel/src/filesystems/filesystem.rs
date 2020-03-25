use crate::files::filename::Path;
use crate::files::handle::LocalHandle;

pub trait FileSystem {
  fn open(&self, path: &Path) -> Result<LocalHandle, ()>;
  fn read(&self, handle: LocalHandle, buffer: &mut [u8]) -> Result<usize, ()>;
  fn write(&self, handle: LocalHandle, buffer: &[u8]) -> Result<usize, ()>;
  fn close(&self, handle: LocalHandle) -> Result<(), ()>;
}