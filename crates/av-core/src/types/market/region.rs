/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Region string normalization for AlphaVantage responses.
//!
//! AlphaVantage's `SYMBOL_SEARCH` and related endpoints return human-readable
//! region names like `"United States"` (13 chars) and `"United Kingdom"`
//! (14 chars). Several `symbols`-table-adjacent columns are `VARCHAR(10)`,
//! so the long names must be mapped to short codes before insertion.
//!
//! This used to live in `av-cli` (private to the loader binary). It was
//! promoted here so non-CLI consumers (the av-web UI) can use the same
//! mapping table without duplicating it. `av-core` is a low-dependency
//! crate, so the function is silent on truncation — wrap it if you want
//! a `tracing::warn!` at the call site.

/// Maps AlphaVantage's verbose region strings to short codes that fit
/// `VARCHAR(10)`. Unknown regions pass through unchanged, then are
/// truncated (UTF-8-safely, by codepoint count) to 10 characters so they
/// still fit the column.
///
/// | AlphaVantage region                       | Returned |
/// |-------------------------------------------|----------|
/// | `"United States"`                         | `"USA"`  |
/// | `"United Kingdom"`                        | `"UK"`   |
/// | `"Toronto"`, `"Toronto Venture"`          | `"TOR"`  |
/// | `"India"`, `"India/Bombay"`, `"Bombay"`   | `"Bomb"` |
/// | `"Brazil"`, `"Sao Paolo"`, etc.           | `"SaoP"` |
/// | (full list inside)                        |          |
pub fn normalize_alpha_region(region: &str) -> String {
  let normalized = match region {
    "United States" => "USA",
    "United Kingdom" => "UK",
    "Frankfurt" => "Frank",
    "Toronto" | "Toronto Venture" => "TOR",
    "India/Bombay" | "India" | "Bombay" => "Bomb",
    "Brazil/Sao Paolo" | "Brazil" | "Sao Paolo" => "SaoP",
    "Amsterdam" => "AMS",
    "XETRA" => "XETRA",
    "Shanghai" => "SH",
    "Hong Kong" => "HK",
    "Tokyo" => "TYO",
    "London" => "LON",
    "Paris" => "PAR",
    "Singapore" => "SG",
    "Sydney" => "SYD",
    "Mexico" => "MEX",
    "Canada" => "CAN",
    "Germany" => "DE",
    "Switzerland" => "CH",
    "Japan" => "JP",
    "Australia" => "AU",
    "Netherlands" => "NL",
    _ => region,
  };

  // PostgreSQL's VARCHAR(n) limit counts characters (codepoints), not bytes.
  // `&s[..10]` would panic on a multi-byte split, so use chars().
  if normalized.chars().count() > 10 {
    normalized.chars().take(10).collect()
  } else {
    normalized.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn known_regions_map_to_short_codes() {
    assert_eq!(normalize_alpha_region("United States"), "USA");
    assert_eq!(normalize_alpha_region("United Kingdom"), "UK");
    assert_eq!(normalize_alpha_region("Hong Kong"), "HK");
    assert_eq!(normalize_alpha_region("Brazil/Sao Paolo"), "SaoP");
  }

  #[test]
  fn short_unknown_region_passes_through() {
    assert_eq!(normalize_alpha_region("Oslo"), "Oslo");
  }

  #[test]
  fn long_unknown_region_is_truncated_safely() {
    let out = normalize_alpha_region("Cairo/Egypt Stock Exchange");
    assert_eq!(out.chars().count(), 10);
  }
}
