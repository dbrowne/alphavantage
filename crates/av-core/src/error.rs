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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("Environment variable error: {0}")]
  EnvVar(#[from] std::env::VarError),

  #[error("Configuration error: {0}")]
  Config(String),

  #[error("Failed to retrieve API key")]
  ApiKey(String),

  #[error("Serialization error")]
  Serde(#[from] serde_json::Error),

  #[error("Date parsing error")]
  ParseDate(#[from] chrono::ParseError),

  #[error("Missing required field: {0}")]
  MissingField(String),

  #[error("Rate limit exceeded: {0}")]
  RateLimit(String),

  #[error("Invalid API response: {0}")]
  InvalidResponse(String),

  #[error("Unexpected error: {0}")]
  Unexpected(String),

  #[error("HTTP error: {0}")]
  Http(String),

  #[error("API error: {0}")]
  Api(String),

  #[error("Parse error: {0}")]
  Parse(String),
}

pub type Result<T> = std::result::Result<T, Error>;
