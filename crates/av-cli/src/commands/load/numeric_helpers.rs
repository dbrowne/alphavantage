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

//! Safe `f64` → [`BigDecimal`] conversion helpers with precision clamping.
//!
//! API responses from CoinGecko, CoinMarketCap, and similar sources return
//! numeric data as `f64`, but the local PostgreSQL schema stores them as
//! fixed-precision `NUMERIC(p, s)` columns. Naive conversion can fail in two
//! ways:
//!
//! 1. **Overflow** — Values exceeding the column's precision (e.g., a
//!    `circulating_supply` of `1e25` won't fit in `NUMERIC(30, 8)` which
//!    only allows 22 digits before the decimal).
//! 2. **Non-finite values** — `NaN` and `±Infinity` cannot be represented as
//!    `BigDecimal` and would crash a naive `from_str` call.
//!
//! This module provides three layered conversion functions:
//!
//! - [`f64_to_bigdecimal_clamped`] — The general-purpose primitive that
//!   accepts an explicit precision limit, handles non-finite values, and
//!   clamps overflow with a warning log.
//! - [`f64_to_price_bigdecimal`] — Convenience wrapper for `NUMERIC(20, 8)`
//!   columns (12 digits before decimal). Used for prices, ATH/ATL, percentage
//!   changes.
//! - [`f64_to_supply_bigdecimal`] — Convenience wrapper for `NUMERIC(30, 8)`
//!   columns (22 digits before decimal). Used for token supply values.
//!
//! ## Conversion Strategy
//!
//! The clamped value is converted via `format!("{:.8}", clamped_value)` →
//! `BigDecimal::from_str` to avoid scientific notation issues that can occur
//! when very large or very small floats are formatted with the default `{}`.
//!
//! ## Logging
//!
//! Both clamping and non-finite rejections log a warning via `tracing::warn`
//! that includes the SID and field name for diagnostic purposes. SID 0 may
//! be used as a sentinel when the caller doesn't have a specific symbol context.
//!
//! ## Consumed By
//!
//! - [`super::crypto_overview::save_crypto_overviews_with_github_to_db`] —
//!   Converts price, supply, ATH, ATL, and percent change values for the
//!   `crypto_overview_basic` and `crypto_overview_metrics` tables.
//! - Any other loader that needs to safely convert API-returned floats to
//!   the database's `NUMERIC` columns.

use bigdecimal::BigDecimal;
use std::str::FromStr;
use tracing::warn;

/// Safely converts an `f64` to [`BigDecimal`] with precision clamping for
/// `NUMERIC(precision, scale)` columns.
///
/// This is the general-purpose primitive used by [`f64_to_price_bigdecimal`]
/// and [`f64_to_supply_bigdecimal`]. Three failure modes are handled:
///
/// 1. **Non-finite** — `NaN`, `Infinity`, and `-Infinity` return `None` after
///    logging a warning.
/// 2. **Overflow** — Values exceeding the precision limit are clamped to the
///    maximum representable value (preserving sign) and a warning is logged.
/// 3. **Conversion failure** — The unlikely case of a `BigDecimal::from_str`
///    failure on a clamped float returns `None`.
///
/// ## Maximum Value Calculation
///
/// The max representable value is computed based on `max_digits_before_decimal`:
///
/// - **≥ 22 digits** → `9.99999999e21` (just under `10^22`, safe for `NUMERIC(30, 8)`)
/// - **≥ 12 digits** → `9.99999999e11` (just under `10^12`, safe for `NUMERIC(20, 8)`)
/// - **otherwise** → `10^max_digits_before_decimal × 0.9999`
///
/// Using values "just under" `10^N` rather than `10^N - 1` avoids edge cases
/// where rounding pushes the BigDecimal representation over the column limit.
///
/// # Arguments
///
/// * `value` — The `f64` value to convert.
/// * `max_digits_before_decimal` — The number of digits the target column
///   allows before the decimal point (e.g., 22 for `NUMERIC(30, 8)`).
/// * `field_name` — Field name for warning logs (e.g., `"circulating_supply"`).
/// * `sid` — Symbol ID for warning logs (use `0` as a sentinel when no
///   specific symbol applies).
///
/// # Returns
///
/// * `Some(BigDecimal)` — Successfully converted (possibly clamped) value.
/// * `None` — Value was non-finite or string conversion failed.
pub fn f64_to_bigdecimal_clamped(
  value: f64,
  max_digits_before_decimal: u32,
  field_name: &str,
  sid: i64,
) -> Option<BigDecimal> {
  // Check for non-finite values
  if !value.is_finite() {
    warn!("SID {}: {} has non-finite value ({}), skipping", sid, field_name, value);
    return None;
  }

  // Calculate maximum representable value for the precision
  // For NUMERIC(30,8), we need max 22 digits before decimal
  // Use 9.999...e21 instead of 10^22 to stay safely under the limit
  let max_value = if max_digits_before_decimal >= 22 {
    9.99999999e21 // Just under 10^22, safe for NUMERIC(30,8)
  } else if max_digits_before_decimal >= 12 {
    9.99999999e11 // Just under 10^12, safe for NUMERIC(20,8)
  } else {
    10_f64.powi(max_digits_before_decimal as i32) * 0.9999
  };

  // Clamp the value if it exceeds the maximum
  let clamped_value = if value.abs() >= max_value {
    let clamped = if value > 0.0 { max_value } else { -max_value };
    warn!(
      "SID {}: {} value ({}) exceeds precision limit (max {} digits), clamping to {}",
      sid, field_name, value, max_digits_before_decimal, clamped
    );
    clamped
  } else {
    value
  };

  // Convert to BigDecimal
  // Use a more careful conversion to avoid scientific notation issues
  BigDecimal::from_str(&format!("{:.8}", clamped_value)).ok()
}

/// Converts an `f64` to [`BigDecimal`] for `NUMERIC(20, 8)` columns.
///
/// Suitable for prices, percentage changes, ATH/ATL values, and other
/// "price-scale" data where 12 digits before the decimal (~`9.99e11`) is
/// sufficient. Delegates to [`f64_to_bigdecimal_clamped`] with
/// `max_digits_before_decimal = 12`.
///
/// Values exceeding the limit are clamped (with warning); non-finite values
/// return `None`.
pub fn f64_to_price_bigdecimal(value: f64, field_name: &str, sid: i64) -> Option<BigDecimal> {
  f64_to_bigdecimal_clamped(value, 12, field_name, sid)
}

/// Converts an `f64` to [`BigDecimal`] for `NUMERIC(30, 8)` columns.
///
/// Suitable for token supply values (circulating, total, max supply) where
/// the much larger range (~`9.99e21`) is needed to accommodate cryptocurrencies
/// with very high supply counts (e.g., SHIB at ~`5.8e14`). Delegates to
/// [`f64_to_bigdecimal_clamped`] with `max_digits_before_decimal = 22`.
///
/// Values exceeding the limit are clamped (with warning); non-finite values
/// return `None`.
pub fn f64_to_supply_bigdecimal(value: f64, field_name: &str, sid: i64) -> Option<BigDecimal> {
  f64_to_bigdecimal_clamped(value, 22, field_name, sid)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normal_conversion() {
    let result = f64_to_price_bigdecimal(123.456, "price", 1);
    assert!(result.is_some());
    let bd = result.unwrap();
    assert_eq!(bd.to_string(), "123.45600000");
  }

  #[test]
  fn test_large_supply_within_limits() {
    // 1 trillion - should be fine for NUMERIC(30,8) which allows up to 10^22
    let result = f64_to_supply_bigdecimal(1_000_000_000_000.0, "circulating_supply", 123);
    assert!(result.is_some());
  }

  #[test]
  fn test_supply_clamping() {
    // 10^23 - exceeds NUMERIC(30,8) limit, should be clamped
    let result = f64_to_supply_bigdecimal(1e23, "circulating_supply", 456);
    assert!(result.is_some());
    // Should be clamped to max value
    let bd = result.unwrap();
    assert!(bd < BigDecimal::from_str("10000000000000000000000").unwrap());
  }

  #[test]
  fn test_price_clamping() {
    // 10^13 - exceeds NUMERIC(20,8) limit, should be clamped
    let result = f64_to_price_bigdecimal(1e13, "price", 789);
    assert!(result.is_some());
    // Should be clamped to max value (10^12 - 1)
    let bd = result.unwrap();
    assert!(bd < BigDecimal::from_str("1000000000000").unwrap());
  }

  #[test]
  fn test_non_finite_values() {
    assert!(f64_to_price_bigdecimal(f64::NAN, "price", 1).is_none());
    assert!(f64_to_price_bigdecimal(f64::INFINITY, "price", 1).is_none());
    assert!(f64_to_price_bigdecimal(f64::NEG_INFINITY, "price", 1).is_none());
  }

  #[test]
  fn test_negative_values() {
    let result = f64_to_price_bigdecimal(-123.456, "price_change", 1);
    assert!(result.is_some());
    let bd = result.unwrap();
    assert_eq!(bd.to_string(), "-123.45600000");
  }
}
