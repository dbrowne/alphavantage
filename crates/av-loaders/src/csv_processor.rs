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

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;
  use tempfile::NamedTempFile;

  fn create_temp_csv(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
  }

  #[test]
  fn test_csv_processor_new() {
    let processor = CsvProcessor::new();
    drop(processor);
  }

  #[test]
  fn test_csv_processor_default() {
    let processor = CsvProcessor::default();
    drop(processor);
  }

  #[test]
  fn test_parse_symbol_list_simple() {
    let csv_content = "Symbol,Name\nAAPL,Apple Inc\nMSFT,Microsoft\nGOOG,Alphabet";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 3);
    assert!(symbols.contains(&"AAPL".to_string()));
    assert!(symbols.contains(&"MSFT".to_string()));
    assert!(symbols.contains(&"GOOG".to_string()));
  }

  #[test]
  fn test_parse_symbol_list_filters_test_issues() {
    let csv_content = "Symbol,Name\nAAPL,Apple Inc\nTEST,Test Issue\nMSFT,Microsoft";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
    assert!(symbols.contains(&"AAPL".to_string()));
    assert!(symbols.contains(&"MSFT".to_string()));
    assert!(!symbols.iter().any(|s| s.contains("TEST")));
  }

  #[test]
  fn test_parse_symbol_list_trims_whitespace() {
    let csv_content = "Symbol,Name\n  AAPL  ,Apple Inc\n MSFT ,Microsoft";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
    assert!(symbols.contains(&"AAPL".to_string()));
    assert!(symbols.contains(&"MSFT".to_string()));
  }

  #[test]
  fn test_parse_symbol_list_skips_empty() {
    let csv_content = "Symbol,Name\nAAPL,Apple\n,Empty\nMSFT,Microsoft";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
  }

  #[test]
  fn test_parse_symbol_list_lowercase_header() {
    let csv_content = "symbol,name\nAAPL,Apple Inc";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 1);
    assert!(symbols.contains(&"AAPL".to_string()));
  }

  #[test]
  fn test_parse_symbol_list_no_symbol_header() {
    // Falls back to first column (index 0)
    let csv_content = "ticker,name\nAAPL,Apple Inc\nMSFT,Microsoft";
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_symbol_list(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
    assert!(symbols.contains(&"AAPL".to_string()));
  }

  #[test]
  fn test_parse_symbol_list_file_not_found() {
    let processor = CsvProcessor::new();
    let result = processor.parse_symbol_list("/nonexistent/path/file.csv");
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_nasdaq_listed() {
    let csv_content = r#"Symbol,Security Name,Market Category,Test Issue,Financial Status,Round Lot Size,ETF,NextShares
AAPL,Apple Inc. - Common Stock,Q,N,N,100,N,N
MSFT,Microsoft Corporation - Common Stock,Q,N,N,100,N,N
SPY,SPDR S&P 500 ETF Trust,G,N,N,100,Y,N"#;
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_nasdaq_listed(file.path()).unwrap();

    assert_eq!(symbols.len(), 3);

    let aapl = symbols.iter().find(|s| s.symbol == "AAPL").unwrap();
    assert_eq!(aapl.name, "Apple Inc. - Common Stock");
    assert!(!aapl.is_etf);

    let spy = symbols.iter().find(|s| s.symbol == "SPY").unwrap();
    assert!(spy.is_etf);
  }

  #[test]
  fn test_parse_nasdaq_listed_filters_test_issues() {
    let csv_content = r#"Symbol,Security Name,Market Category,Test Issue,Financial Status,Round Lot Size,ETF,NextShares
AAPL,Apple Inc,Q,N,N,100,N,N
ZZZZ,Test Security,Q,Y,N,100,N,N
MSFT,Microsoft,Q,N,N,100,N,N"#;
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_nasdaq_listed(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
    assert!(symbols.iter().all(|s| s.symbol != "ZZZZ"));
  }

  #[test]
  fn test_parse_other_listed() {
    let csv_content = r#"ACT Symbol,Security Name,Exchange,CQS Symbol,ETF,Round Lot Size,Test Issue,NASDAQ Symbol
IBM,International Business Machines,N,IBM,N,100,N,IBM
BA,Boeing Company,N,BA,N,100,N,BA
VTI,Vanguard Total Stock Market ETF,A,VTI,Y,100,N,VTI"#;
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_other_listed(file.path()).unwrap();

    assert_eq!(symbols.len(), 3);

    let ibm = symbols.iter().find(|s| s.symbol == "IBM").unwrap();
    assert_eq!(ibm.name, "International Business Machines");
    assert_eq!(ibm.exchange, "N");
    assert!(!ibm.is_etf);

    let vti = symbols.iter().find(|s| s.symbol == "VTI").unwrap();
    assert!(vti.is_etf);
    assert_eq!(vti.exchange, "A");
  }

  #[test]
  fn test_parse_other_listed_filters_test_issues() {
    let csv_content = r#"ACT Symbol,Security Name,Exchange,CQS Symbol,ETF,Round Lot Size,Test Issue,NASDAQ Symbol
IBM,IBM Corp,N,IBM,N,100,N,IBM
TEST,Test Issue,N,TEST,N,100,Y,TEST
BA,Boeing,N,BA,N,100,N,BA"#;
    let file = create_temp_csv(csv_content);

    let processor = CsvProcessor::new();
    let symbols = processor.parse_other_listed(file.path()).unwrap();

    assert_eq!(symbols.len(), 2);
    assert!(symbols.iter().all(|s| s.symbol != "TEST"));
  }

  #[test]
  fn test_nasdaq_symbol_debug() {
    let symbol =
      NasdaqSymbol { symbol: "AAPL".to_string(), name: "Apple Inc".to_string(), is_etf: false };
    let debug_str = format!("{:?}", symbol);
    assert!(debug_str.contains("NasdaqSymbol"));
    assert!(debug_str.contains("AAPL"));
  }

  #[test]
  fn test_other_symbol_debug() {
    let symbol = OtherSymbol {
      symbol: "IBM".to_string(),
      name: "IBM Corp".to_string(),
      exchange: "NYSE".to_string(),
      is_etf: false,
    };
    let debug_str = format!("{:?}", symbol);
    assert!(debug_str.contains("OtherSymbol"));
    assert!(debug_str.contains("IBM"));
    assert!(debug_str.contains("NYSE"));
  }
}
