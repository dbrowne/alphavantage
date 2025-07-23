//! Process tracking for ETL job monitoring
//!
//! Tracks the state of data loading processes in the database:
//! - Process start time
//! - Process completion state (success, failed, completed with errors)
//! - Process duration
//!
//! This enables monitoring of long-running ETL jobs and provides
//! the ability to restart failed processes from where they left off.

use chrono::{DateTime, Utc};
use av_database::models::{NewProcState, ProcState, ProcType};
use crate::{LoaderError, LoaderResult};

#[derive(Debug, Clone, Copy)]
pub enum ProcessState {
    Running,
    Success,
    Failed,
    CompletedWithErrors,
}

impl ProcessState {
    pub fn to_state_id(&self) -> i32 {
        match self {
            ProcessState::Running => 1,
            ProcessState::Success => 2,
            ProcessState::Failed => 3,
            ProcessState::CompletedWithErrors => 4,
        }
    }
}

pub struct ProcessTracker {
    proc_type_id: i32,
    spid: Option<i32>,
    start_time: DateTime<Utc>,
}

impl ProcessTracker {
    pub async fn new(
        conn: &mut av_database::DbConnection,
        process_name: &str,
    ) -> LoaderResult<Self> {
        // Get or create process type
        let proc_type = ProcType::find_or_create(conn, process_name).await
            .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;

        Ok(Self {
            proc_type_id: proc_type.id,
            spid: None,
            start_time: Utc::now(),
        })
    }

    pub async fn start(
        &mut self,
        _process_name: &str,
    ) -> LoaderResult<()> {
        // Process is started on creation
        Ok(())
    }

    pub async fn log_start(
        &mut self,
        conn: &mut av_database::DbConnection,
    ) -> LoaderResult<i32> {
        let new_state = NewProcState {
            proc_id: self.proc_type_id,
            start_time: self.start_time,
            end_state: None,
            end_time: None,
        };

        let state = new_state.insert(conn).await
            .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;

        self.spid = Some(state.spid);
        Ok(state.spid)
    }

    pub async fn complete(
        &self,
        state: ProcessState,
    ) -> LoaderResult<()> {
        // Completion is logged separately
        Ok(())
    }

    pub async fn log_complete(
        &self,
        conn: &mut av_database::DbConnection,
        state: ProcessState,
    ) -> LoaderResult<()> {
        if let Some(spid) = self.spid {
            ProcState::update_end_state(
                conn,
                spid,
                state.to_state_id(),
                Utc::now(),
            ).await
                .map_err(|e| LoaderError::ProcessTrackingError(e.to_string()))?;
        }

        Ok(())
    }
}