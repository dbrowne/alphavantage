// Helper functions for safe numeric conversions to BigDecimal with precision limits

use bigdecimal::BigDecimal;
use std::str::FromStr;
use tracing::warn;

/// Safely convert f64 to BigDecimal with precision limits for NUMERIC(precision, scale)
/// If the value exceeds the maximum representable value, it's clamped to the max
/// and a warning is logged.
///
/// # Arguments
/// * `value` - The f64 value to convert
/// * `max_digits_before_decimal` - Maximum digits before decimal point (e.g., 22 for NUMERIC(30,8))
/// * `field_name` - Name of the field for logging purposes
/// * `sid` - Symbol ID being processed for logging purposes
///
/// # Returns
/// * `Some(BigDecimal)` - Successfully converted and validated value (clamped if necessary)
/// * `None` - If value is not finite (NaN, Infinity)
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

/// Convert f64 to BigDecimal for NUMERIC(20,8) fields (prices, percentages, etc.)
/// Max 12 digits before decimal point
pub fn f64_to_price_bigdecimal(value: f64, field_name: &str, sid: i64) -> Option<BigDecimal> {
  f64_to_bigdecimal_clamped(value, 12, field_name, sid)
}

/// Convert f64 to BigDecimal for NUMERIC(30,8) fields (supply values)
/// Max 22 digits before decimal point
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
