use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use super::process_state::{ProcessID, ProcessState};

/**
 * Mapping of PIDs to process structures.
 * Also keeps track of the "current" process.
 */
pub struct ProcessMap {
  current: ProcessID,
  next_pid: AtomicU32,
  processes: BTreeMap<ProcessID, Arc<ProcessState>>,
}

impl ProcessMap {
  pub fn new() -> ProcessMap {
    ProcessMap {
      current: ProcessID::new(0),
      next_pid: AtomicU32::new(1),
      processes: BTreeMap::new(),
    }
  }

  fn get_next_pid(&self) -> ProcessID {
    let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);
    ProcessID::new(pid)
  }

  pub fn spawn_process(&mut self) -> ProcessID {
    let pid = self.get_next_pid();
    self.processes.insert(pid, Arc::new(ProcessState::new(pid)));
    pid
  }

  pub fn get_process(&self, pid: ProcessID) -> Option<&Arc<ProcessState>> {
    self.processes.get(&pid)
  }

  pub fn get_current_process(&self) -> Option<&Arc<ProcessState>> {
    self.processes.get(&self.current)
  }

  pub fn make_current(&mut self, pid: ProcessID) {
    self.current = pid;
  }
}