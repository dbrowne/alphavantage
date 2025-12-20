/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

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

impl Default for ProcessTracker {
  fn default() -> Self {
    Self::new()
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_process_state_debug() {
    assert!(format!("{:?}", ProcessState::Running).contains("Running"));
    assert!(format!("{:?}", ProcessState::Success).contains("Success"));
    assert!(format!("{:?}", ProcessState::Failed).contains("Failed"));
    assert!(format!("{:?}", ProcessState::CompletedWithErrors).contains("CompletedWithErrors"));
  }

  #[test]
  fn test_process_state_clone() {
    let state = ProcessState::Running;
    let cloned = state.clone();
    assert!(matches!(cloned, ProcessState::Running));
  }

  #[test]
  fn test_process_info_clone() {
    let info = ProcessInfo {
      process_name: "test_process".to_string(),
      start_time: Utc::now(),
      end_time: None,
      state: ProcessState::Running,
      error_message: None,
      records_processed: Some(100),
    };
    let cloned = info.clone();
    assert_eq!(cloned.process_name, "test_process");
    assert_eq!(cloned.records_processed, Some(100));
  }

  #[test]
  fn test_process_info_debug() {
    let info = ProcessInfo {
      process_name: "test".to_string(),
      start_time: Utc::now(),
      end_time: None,
      state: ProcessState::Running,
      error_message: None,
      records_processed: None,
    };
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("ProcessInfo"));
    assert!(debug_str.contains("test"));
  }

  #[test]
  fn test_process_tracker_new() {
    let tracker = ProcessTracker::new();
    // Just verify it can be created
    assert!(true);
    drop(tracker);
  }

  #[test]
  fn test_process_tracker_default() {
    let tracker = ProcessTracker::default();
    drop(tracker);
  }

  #[tokio::test]
  async fn test_process_tracker_start() {
    let tracker = ProcessTracker::new();
    let result = tracker.start("test_job").await;
    assert!(result.is_ok());

    let processes = tracker.get_all().await;
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].process_name, "test_job");
    assert!(matches!(processes[0].state, ProcessState::Running));
    assert!(processes[0].end_time.is_none());
  }

  #[tokio::test]
  async fn test_process_tracker_complete_success() {
    let tracker = ProcessTracker::new();
    tracker.start("test_job").await.unwrap();
    let result = tracker.complete(ProcessState::Success).await;
    assert!(result.is_ok());

    let processes = tracker.get_all().await;
    assert_eq!(processes.len(), 1);
    assert!(matches!(processes[0].state, ProcessState::Success));
    assert!(processes[0].end_time.is_some());
  }

  #[tokio::test]
  async fn test_process_tracker_complete_failed() {
    let tracker = ProcessTracker::new();
    tracker.start("failing_job").await.unwrap();
    tracker.complete(ProcessState::Failed).await.unwrap();

    let processes = tracker.get_all().await;
    assert!(matches!(processes[0].state, ProcessState::Failed));
  }

  #[tokio::test]
  async fn test_process_tracker_complete_with_errors() {
    let tracker = ProcessTracker::new();
    tracker.start("partial_job").await.unwrap();
    tracker.complete(ProcessState::CompletedWithErrors).await.unwrap();

    let processes = tracker.get_all().await;
    assert!(matches!(processes[0].state, ProcessState::CompletedWithErrors));
  }

  #[tokio::test]
  async fn test_process_tracker_multiple_processes() {
    let tracker = ProcessTracker::new();

    tracker.start("job1").await.unwrap();
    tracker.complete(ProcessState::Success).await.unwrap();

    tracker.start("job2").await.unwrap();
    tracker.complete(ProcessState::Failed).await.unwrap();

    tracker.start("job3").await.unwrap();

    let processes = tracker.get_all().await;
    assert_eq!(processes.len(), 3);
    assert_eq!(processes[0].process_name, "job1");
    assert_eq!(processes[1].process_name, "job2");
    assert_eq!(processes[2].process_name, "job3");
  }

  #[tokio::test]
  async fn test_process_tracker_get_all_empty() {
    let tracker = ProcessTracker::new();
    let processes = tracker.get_all().await;
    assert!(processes.is_empty());
  }

  #[tokio::test]
  async fn test_process_tracker_complete_empty_does_not_panic() {
    let tracker = ProcessTracker::new();
    // Complete without starting should not panic
    let result = tracker.complete(ProcessState::Success).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_process_tracker_timing() {
    let tracker = ProcessTracker::new();
    let before = Utc::now();

    tracker.start("timed_job").await.unwrap();

    let processes = tracker.get_all().await;
    let start_time = processes[0].start_time;

    assert!(start_time >= before);

    tracker.complete(ProcessState::Success).await.unwrap();

    let processes = tracker.get_all().await;
    let end_time = processes[0].end_time.unwrap();

    assert!(end_time >= start_time);
  }
}
