#![allow(async_fn_in_trait)] // Add this at the top of the file

pub mod crypto;
pub mod forex;
pub mod fundamentals;
pub mod news;
pub mod time_series;
pub mod crypto_social;

use crate::transport::Transport;
use av_core::Result;
use std::sync::Arc;

/// Base trait for all endpoint implementations
pub trait EndpointBase: Send + Sync {
  /// Wait for rate limit before making a request
  async fn wait_for_rate_limit(&self) -> Result<()>; // Keep it simple with async fn

  /// Get the transport instance
  fn transport(&self) -> &Arc<Transport>;
}

/// Macro to implement common endpoint functionality
#[macro_export]
macro_rules! impl_endpoint_base {
  ($endpoint:ty) => {
    impl EndpointBase for $endpoint {
      async fn wait_for_rate_limit(&self) -> Result<()> {
        self.rate_limiter.until_ready().await;
        Ok(())
      }

      fn transport(&self) -> &Arc<Transport> {
        &self.transport
      }
    }
  };
}
