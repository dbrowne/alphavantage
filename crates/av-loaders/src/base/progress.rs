/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Progress bar utilities for loaders.

use indicatif::{ProgressBar, ProgressStyle as IndicatifStyle};
use std::sync::Arc;

/// Pre-defined progress bar styles for consistent appearance.
#[derive(Debug, Clone, Copy, Default)]
pub enum ProgressStyle {
  /// Standard progress bar with elapsed time, bar, and position.
  /// Format: `[00:00:05] [████████████░░░░░░░░] 50/100 Loading...`
  #[default]
  Standard,

  /// Compact progress bar without elapsed time.
  /// Format: `[████████████░░░░░░░░] 50/100`
  Compact,

  /// Detailed progress bar with spinner and message.
  /// Format: `⠋ [00:00:05] [████████████░░░░░░░░] 50/100 Processing item xyz`
  Detailed,

  /// Simple spinner without bar (for unknown total).
  /// Format: `⠋ Processing... (50 items)`
  Spinner,
}

impl ProgressStyle {
  /// Get the indicatif template string for this style.
  fn template(&self) -> &'static str {
    match self {
      Self::Standard => "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
      Self::Compact => "[{bar:40.cyan/blue}] {pos}/{len}",
      Self::Detailed => {
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}"
      }
      Self::Spinner => "{spinner:.green} {msg} ({pos} items)",
    }
  }

  /// Get the progress characters for this style.
  fn progress_chars(&self) -> &'static str {
    match self {
      Self::Standard | Self::Compact | Self::Detailed => "#>-",
      Self::Spinner => "",
    }
  }

  /// Convert to an indicatif ProgressStyle.
  fn to_indicatif(&self) -> IndicatifStyle {
    let style = IndicatifStyle::default_bar().template(self.template()).unwrap();

    let chars = self.progress_chars();
    if !chars.is_empty() { style.progress_chars(chars) } else { style }
  }
}

/// Manages progress bar creation and lifecycle.
///
/// This provides a consistent way to create and manage progress bars
/// across loaders, respecting the `show_progress` configuration option.
///
/// # Example
///
/// ```ignore
/// use av_loaders::base::{ProgressManager, ProgressStyle};
///
/// // Create a progress bar if show_progress is true
/// let progress = ProgressManager::new_optional(
///     100,
///     context.config.show_progress,
///     ProgressStyle::Standard,
///     Some("Loading items"),
/// );
///
/// // Use in async loop
/// for item in items {
///     // ... process item ...
///     ProgressManager::increment(&progress);
/// }
///
/// // Finish with message
/// ProgressManager::finish(&progress, "Completed loading 100 items");
/// ```
pub struct ProgressManager;

impl ProgressManager {
  /// Create a new progress bar.
  ///
  /// # Arguments
  ///
  /// * `total` - Total number of items to process
  /// * `style` - Visual style of the progress bar
  /// * `message` - Optional initial message
  pub fn new(total: u64, style: ProgressStyle, message: Option<&str>) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(style.to_indicatif());
    if let Some(msg) = message {
      pb.set_message(msg.to_string());
    }
    pb
  }

  /// Create an Arc-wrapped progress bar for sharing across async tasks.
  pub fn new_shared(total: u64, style: ProgressStyle, message: Option<&str>) -> Arc<ProgressBar> {
    Arc::new(Self::new(total, style, message))
  }

  /// Create an optional progress bar based on configuration.
  ///
  /// Returns `Some(Arc<ProgressBar>)` if `show_progress` is true, `None` otherwise.
  pub fn new_optional(
    total: u64,
    show_progress: bool,
    style: ProgressStyle,
    message: Option<&str>,
  ) -> Option<Arc<ProgressBar>> {
    if show_progress { Some(Self::new_shared(total, style, message)) } else { None }
  }

  /// Increment the progress bar by one if it exists.
  pub fn increment(progress: &Option<Arc<ProgressBar>>) {
    if let Some(pb) = progress {
      pb.inc(1);
    }
  }

  /// Increment the progress bar by a specific amount if it exists.
  pub fn increment_by(progress: &Option<Arc<ProgressBar>>, amount: u64) {
    if let Some(pb) = progress {
      pb.inc(amount);
    }
  }

  /// Set the message on the progress bar if it exists.
  pub fn set_message(progress: &Option<Arc<ProgressBar>>, message: &str) {
    if let Some(pb) = progress {
      pb.set_message(message.to_string());
    }
  }

  /// Finish the progress bar with a message if it exists.
  pub fn finish(progress: &Option<Arc<ProgressBar>>, message: &str) {
    if let Some(pb) = progress {
      pb.finish_with_message(message.to_string());
    }
  }

  /// Finish the progress bar and clear it from the terminal.
  pub fn finish_and_clear(progress: &Option<Arc<ProgressBar>>) {
    if let Some(pb) = progress {
      pb.finish_and_clear();
    }
  }

  /// Abandon the progress bar (stop without marking as complete).
  pub fn abandon(progress: &Option<Arc<ProgressBar>>, message: Option<&str>) {
    if let Some(pb) = progress {
      if let Some(msg) = message {
        pb.abandon_with_message(msg.to_string());
      } else {
        pb.abandon();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_progress_style_templates() {
    // Just verify templates are valid
    let _ = ProgressStyle::Standard.to_indicatif();
    let _ = ProgressStyle::Compact.to_indicatif();
    let _ = ProgressStyle::Detailed.to_indicatif();
    let _ = ProgressStyle::Spinner.to_indicatif();
  }

  #[test]
  fn test_new_progress_bar() {
    let pb = ProgressManager::new(100, ProgressStyle::Standard, Some("Testing"));
    assert_eq!(pb.length(), Some(100));
    assert_eq!(pb.position(), 0);
  }

  #[test]
  fn test_new_optional_enabled() {
    let pb = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    assert!(pb.is_some());
  }

  #[test]
  fn test_new_optional_disabled() {
    let pb = ProgressManager::new_optional(100, false, ProgressStyle::Standard, None);
    assert!(pb.is_none());
  }

  #[test]
  fn test_increment() {
    let pb = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::increment(&pb);
    assert_eq!(pb.as_ref().unwrap().position(), 1);
  }

  #[test]
  fn test_increment_none() {
    let pb: Option<Arc<ProgressBar>> = None;
    // Should not panic
    ProgressManager::increment(&pb);
  }

  #[test]
  fn test_set_message() {
    let pb = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::set_message(&pb, "Processing...");
    // Message is set (can't easily verify, but no panic)
  }

  #[test]
  fn test_finish() {
    let pb = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::finish(&pb, "Done!");
    assert!(pb.as_ref().unwrap().is_finished());
  }
}
