pub mod crypto;
pub mod forex;
pub mod fundamentals;
pub mod news;
pub mod time_series;

use crate::transport::Transport;
use av_core::Result;
use governor::{
  RateLimiter,
  clock::DefaultClock,
  middleware::NoOpMiddleware,
  state::{InMemoryState, NotKeyed},
};
use std::sync::Arc;

/// Base trait for endpoint implementations
///
/// Provides common functionality needed by all endpoint modules
pub trait EndpointBase {
  /// Wait for rate limit before making a request
  async fn wait_for_rate_limit(&self) -> Result<()>;

  /// Get a reference to the transport layer
  fn transport(&self) -> &Arc<Transport>;
}

/// Macro to implement the EndpointBase trait for endpoint structs
macro_rules! impl_endpoint_base {
  ($struct_name:ident) => {
    impl EndpointBase for $struct_name {
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

pub(crate) use impl_endpoint_base;

/// Common endpoint structure
///
/// All endpoint modules follow this pattern with a transport layer
/// and rate limiter for consistent behavior.
pub struct EndpointCore {
  pub transport: Arc<Transport>,
  pub rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl EndpointCore {
  /// Create a new endpoint core
  pub fn new(
    transport: Arc<Transport>,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
  ) -> Self {
    Self { transport, rate_limiter }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transport::Transport;
  use governor::{Quota, RateLimiter};
  use std::num::NonZeroU32;

  #[test]
  fn test_endpoint_core_creation() {
    let transport = Arc::new(Transport::new_mock());
    let quota = Quota::per_minute(NonZeroU32::new(75).unwrap());
    let rate_limiter = Arc::new(RateLimiter::direct(quota));

    let core = EndpointCore::new(transport, rate_limiter);

    assert_eq!(core.transport.base_url(), "https://mock.alphavantage.co");
  }
}
