use crate::memory::address::VirtualAddress;
use super::fat::{Cluster, ClusterChain};
use super::file::{FileDate, FileTime, FileType, name_character_matches};

/// Directories are handled internally as chains of Clusters, so that the driver
/// can easily iterate through the sections on disk.
pub struct Directory {
  pub clusters: ClusterChain,
}

impl Directory {
  pub fn empty() -> Directory {
    Directory {
      clusters: ClusterChain::empty(),
    }
  }
}

/// On-disk representation of a file or subdirectory
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct DirectoryEntry {
  /// Short filename
  file_name: [u8; 8],
  /// File extension
  ext: [u8; 3],
  /// File attributes
  attributes: u8,
  /// Reserved byte used for various nonstandard things
  nonstandard_attributes: u8,
  /// Fine resolution of creation time, in 10ms units. Ranges from 0-199
  fine_create_time: u8,
  /// File creation time
  creation_time: FileTime,
  /// File creation date
  creation_date: FileDate,
  /// Last access date
  access_date: FileDate,
  /// Extended attributes
  extended_attributes: u16,
  /// Last modified time
  last_modify_time: FileTime,
  /// Last modified date
  last_modify_date: FileDate,
  /// First cluster of file data
  first_file_cluster: u16,
  /// File size in bytes
  byte_size: u32,
}

impl DirectoryEntry {
  pub fn at_address(addr: VirtualAddress) -> &'static mut DirectoryEntry {
    let ptr = addr.as_usize() as *mut DirectoryEntry;
    unsafe {
      &mut *ptr
    }
  }

  pub fn get_name(&self) -> &[u8] {
    &self.file_name
  }

  pub fn get_ext(&self) -> &[u8] {
    &self.ext
  }

  pub fn get_file_type(&self) -> FileType {
    if self.attributes & 0x08 == 0x08 {
      FileType::VolumeLabel
    } else if self.attributes & 0x10 == 0x10 {
      FileType::Directory
    } else {
      FileType::File
    }
  }

  pub fn get_first_cluster(&self) -> Cluster {
    Cluster::new(self.first_file_cluster as usize)
  }

  pub fn is_empty(&self) -> bool {
    self.file_name[0] == 0
  }

  pub fn copy_name(&self, buffer: &mut [u8; 8]) {
    for i in 0..8 {
      buffer[i] = self.file_name[i];
    }
  }

  pub fn copy_ext(&self, buffer: &mut [u8; 3]) {
    for i in 0..3 {
      buffer[i] = self.ext[i];
    }
  }

  pub fn get_full_name(&self, buffer: &mut [u8; 11]) {
    for i in 0..8 {
      buffer[i] = self.file_name[i]
    }
    for i in 0..3 {
      buffer[8 + i] = self.ext[i]
    }
  }

  pub fn get_byte_size(&self) -> usize {
    self.byte_size as usize
  }

  pub fn name_matches_search(&self, name: &[u8; 8], ext: &[u8; 3]) -> bool {
    for i in 0..8 {
      if !name_character_matches(self.file_name[i], name[i]) {
        return false;
      }
    }
    for i in 0..3 {
      if !name_character_matches(self.ext[i], ext[i]) {
        return false;
      }
    }
    true
  }
}

pub struct DirectoryEntryIterator<'a> {
  start: VirtualAddress,
  max_count: usize,
  current: usize,

  _parent_data: core::marker::PhantomData<&'a ()>,
}

impl<'a> DirectoryEntryIterator<'a> {
  pub fn new(start: VirtualAddress, max_count: usize) -> DirectoryEntryIterator<'a> {
    DirectoryEntryIterator {
      start,
      max_count,
      current: 0,

      _parent_data: core::marker::PhantomData,
    }
  }
}

impl<'a> Iterator for DirectoryEntryIterator<'a> {
  type Item = &'a mut DirectoryEntry;

  fn next(&mut self) -> Option<Self::Item> {
    if self.current >= self.max_count {
      return None;
    }

    let start_ptr = self.start.as_usize() as *mut DirectoryEntry;
    let ptr = unsafe { start_ptr.offset(self.current as isize) };
    let entry = unsafe { &mut *ptr };
    if entry.is_empty() {
      return None;
    }
    self.current += 1;
    Some(entry)
  }
}

/// Reference to an open file or directory on disk
pub struct FileReference {
  dir_entry: DirectoryEntry,
}
