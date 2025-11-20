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

//! CSV file processor for reading symbol lists
//!
//! Supports multiple CSV formats:
//! - NASDAQ listed securities format
//! - Other exchanges listed format
//! - Simple symbol lists (one symbol per line)
//!
//! The processor filters out test issues and handles various
//! CSV quirks like extra whitespace and different column names.

use crate::LoaderResult;
use csv::Reader;
use serde::Deserialize;
use std::fs::File;
use std::path::Path;

pub struct CsvProcessor;

impl Default for CsvProcessor {
  fn default() -> Self {
    Self::new()
  }
}

impl CsvProcessor {
  pub fn new() -> Self {
    Self
  }

  /// Parse a CSV file containing symbol listings (NASDAQ format)
  pub fn parse_symbol_list<P: AsRef<Path>>(&self, path: P) -> LoaderResult<Vec<String>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);

    let mut symbols = Vec::new();

    // Skip header if present
    let headers = reader.headers()?;

    // Find the symbol column (usually first column)
    let symbol_index = headers
      .iter()
      .position(|h| h.to_lowercase().contains("symbol") || h == "Symbol")
      .unwrap_or(0);

    for result in reader.records() {
      let record = result?;
      if let Some(symbol) = record.get(symbol_index) {
        let symbol = symbol.trim().to_string();
        // Skip empty symbols or test issues
        if !symbol.is_empty() && !symbol.contains("TEST") {
          symbols.push(symbol);
        }
      }
    }

    Ok(symbols)
  }

  /// Parse NASDAQ listed securities CSV format
  pub fn parse_nasdaq_listed<P: AsRef<Path>>(&self, path: P) -> LoaderResult<Vec<NasdaqSymbol>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);

    let mut symbols = Vec::new();
    for result in reader.deserialize() {
      let record: NasdaqListedRecord = result?;
      if record.test_issue != "Y" {
        symbols.push(NasdaqSymbol {
          symbol: record.symbol,
          name: record.security_name,
          is_etf: record.etf == "Y",
        });
      }
    }

    Ok(symbols)
  }

  /// Parse other listed securities CSV format
  pub fn parse_other_listed<P: AsRef<Path>>(&self, path: P) -> LoaderResult<Vec<OtherSymbol>> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(file);

    let mut symbols = Vec::new();
    for result in reader.deserialize() {
      let record: OtherListedRecord = result?;
      if record.test_issue != "Y" {
        symbols.push(OtherSymbol {
          symbol: record.act_symbol,
          name: record.security_name,
          exchange: record.exchange,
          is_etf: record.etf == "Y",
        });
      }
    }

    Ok(symbols)
  }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct NasdaqListedRecord {
  #[serde(rename = "Symbol")]
  symbol: String,

  #[serde(rename = "Security Name")]
  security_name: String,

  #[serde(rename = "Market Category")]
  market_category: String,

  #[serde(rename = "Test Issue")]
  test_issue: String,

  #[serde(rename = "Financial Status")]
  financial_status: String,

  #[serde(rename = "Round Lot Size")]
  round_lot_size: String,

  #[serde(rename = "ETF")]
  etf: String,

  #[serde(rename = "NextShares")]
  next_shares: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OtherListedRecord {
  #[serde(rename = "ACT Symbol")]
  act_symbol: String,

  #[serde(rename = "Security Name")]
  security_name: String,

  #[serde(rename = "Exchange")]
  exchange: String,

  #[serde(rename = "CQS Symbol")]
  cqs_symbol: String,

  #[serde(rename = "ETF")]
  etf: String,

  #[serde(rename = "Round Lot Size")]
  round_lot_size: String,

  #[serde(rename = "Test Issue")]
  test_issue: String,

  #[serde(rename = "NASDAQ Symbol")]
  nasdaq_symbol: String,
}

#[derive(Debug)]
pub struct NasdaqSymbol {
  pub symbol: String,
  pub name: String,
  pub is_etf: bool,
}

#[derive(Debug)]
pub struct OtherSymbol {
  pub symbol: String,
  pub name: String,
  pub exchange: String,
  pub is_etf: bool,
}
