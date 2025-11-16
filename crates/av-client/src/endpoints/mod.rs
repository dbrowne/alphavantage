/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-dot-]browne[-at-]dwightjbrowne[-dot-]com
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

#![allow(async_fn_in_trait)]

pub mod crypto;
pub mod crypto_social;
pub mod forex;
pub mod fundamentals;
pub mod news;
pub mod time_series;

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
