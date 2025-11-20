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
