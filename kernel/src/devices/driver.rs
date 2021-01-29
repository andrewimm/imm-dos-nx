use crate::files::cursor::SeekMethod;

pub trait DeviceDriver {
  #![allow(unused_variables)]

  fn open(&self) -> Result<usize, ()>;

  fn read(&self, index: usize, buffer: &mut [u8]) -> Result<usize, ()>;

  fn write(&self, index: usize, buffer: &[u8]) -> Result<usize, ()>;

  fn close(&self, index: usize) -> Result<(), ()>;

  fn seek(&self, index: usize, offset: SeekMethod) -> Result<usize, ()> {
    Err(())
  }
}

pub type DeviceDriverType = dyn DeviceDriver + Sync + Send;
