//! Common types used across the API

use serde::{Deserialize, Serialize};

/// Time series interval for intraday data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interval {
  #[serde(rename = "1min")]
  OneMin,
  #[serde(rename = "5min")]
  FiveMin,
  #[serde(rename = "15min")]
  FifteenMin,
  #[serde(rename = "30min")]
  ThirtyMin,
  #[serde(rename = "60min")]
  SixtyMin,
}

impl Interval {
  pub fn as_str(&self) -> &'static str {
    match self {
      Interval::OneMin => "1min",
      Interval::FiveMin => "5min",
      Interval::FifteenMin => "15min",
      Interval::ThirtyMin => "30min",
      Interval::SixtyMin => "60min",
    }
  }
}

/// Output size for time series data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputSize {
  #[serde(rename = "compact")]
  Compact,
  #[serde(rename = "full")]
  Full,
}

/// Data type for API responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
  #[serde(rename = "json")]
  Json,
  #[serde(rename = "csv")]
  Csv,
}

/// Region normalization
pub fn normalize_alpha_region(region: &str) -> String {
  match region {
    "United States" => "USA",
    "United Kingdom" => "UK",
    "Frankfurt" => "Frank",
    "Toronto Venture" => "TOR",
    "India/Bombay" => "Bomb",
    "Brazil/Sao Paolo" => "SaoP",
    _ => region,
  }
  .to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize_alpha_region_known() {
    assert_eq!(normalize_alpha_region("United States"), "USA");
    assert_eq!(normalize_alpha_region("United Kingdom"), "UK");
    assert_eq!(normalize_alpha_region("Frankfurt"), "Frank");
    assert_eq!(normalize_alpha_region("Toronto Venture"), "TOR");
    assert_eq!(normalize_alpha_region("India/Bombay"), "Bomb");
    assert_eq!(normalize_alpha_region("Brazil/Sao Paolo"), "SaoP");
  }

  #[test]
  fn test_normalize_alpha_region_unknown() {
    assert_eq!(normalize_alpha_region("Whatever"), "Whatever");
    assert_eq!(normalize_alpha_region("Mars"), "Mars");
    assert_eq!(normalize_alpha_region(""), "");
  }
}
