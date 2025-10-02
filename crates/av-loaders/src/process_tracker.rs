//! Process tracking for monitoring ETL jobs
//! This version uses in-memory tracking instead of database

use crate::LoaderResult;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum ProcessState {
  Running,
  Success,
  Failed,
  CompletedWithErrors,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
  pub process_name: String,
  pub start_time: DateTime<Utc>,
  pub end_time: Option<DateTime<Utc>>,
  pub state: ProcessState,
  pub error_message: Option<String>,
  pub records_processed: Option<usize>,
}

/// In-memory process tracker
pub struct ProcessTracker {
  processes: Arc<Mutex<Vec<ProcessInfo>>>,
}

impl ProcessTracker {
  pub fn new() -> Self {
    Self { processes: Arc::new(Mutex::new(Vec::new())) }
  }

  pub async fn start(&self, process_name: &str) -> LoaderResult<()> {
    let mut processes = self.processes.lock().await;
    processes.push(ProcessInfo {
      process_name: process_name.to_string(),
      start_time: Utc::now(),
      end_time: None,
      state: ProcessState::Running,
      error_message: None,
      records_processed: None,
    });
    Ok(())
  }

  pub async fn complete(&self, state: ProcessState) -> LoaderResult<()> {
    let mut processes = self.processes.lock().await;
    if let Some(last) = processes.last_mut() {
      last.state = state;
      last.end_time = Some(Utc::now());
    }
    Ok(())
  }

  pub async fn get_all(&self) -> Vec<ProcessInfo> {
    self.processes.lock().await.clone()
  }
}
