/*
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 */

//! Progress bar management utilities for loaders.

use indicatif::ProgressBar;
use std::sync::Arc;

/// Progress bar style options for loaders.
#[derive(Debug, Clone, Copy, Default)]
pub enum ProgressStyle {
  /// Standard progress bar with elapsed time, bar, position, and message
  #[default]
  Standard,
  /// Compact progress bar (shorter format)
  Compact,
  /// Detailed progress bar with ETA
  Detailed,
  /// Spinner style for indeterminate progress
  Spinner,
}

impl ProgressStyle {
  /// Get the indicatif template string for this style
  pub fn template(&self) -> &'static str {
    match self {
      ProgressStyle::Standard => {
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"
      }
      ProgressStyle::Compact => "{bar:20.green/white} {pos}/{len} {msg}",
      ProgressStyle::Detailed => {
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} ({eta}) {msg}"
      }
      ProgressStyle::Spinner => "{spinner:.green} [{elapsed_precise}] {msg}",
    }
  }

  /// Get the progress characters for this style
  pub fn progress_chars(&self) -> &'static str {
    match self {
      ProgressStyle::Standard | ProgressStyle::Detailed => "##-",
      ProgressStyle::Compact => "=>-",
      ProgressStyle::Spinner => "",
    }
  }
}

/// Helper for creating and managing optional progress bars.
///
/// This provides a consistent way to create progress bars across loaders
/// while respecting the `show_progress` configuration setting.
///
/// # Example
///
/// ```ignore
/// use loader_base::{ProgressManager, ProgressStyle};
///
/// // Create optional progress bar based on config
/// let progress = ProgressManager::new_optional(
///     100,
///     config.show_progress,
///     ProgressStyle::Standard,
///     Some("Loading items"),
/// );
///
/// // Use helper methods that handle None gracefully
/// ProgressManager::increment(&progress);
/// ProgressManager::set_message(&progress, "Processing item 1");
/// ProgressManager::finish(&progress, "Complete!");
/// ```
pub struct ProgressManager;

impl ProgressManager {
  /// Create a new progress bar, or None if progress is disabled.
  ///
  /// # Arguments
  ///
  /// * `total` - Total number of items to process
  /// * `show_progress` - Whether to actually create the progress bar
  /// * `style` - The visual style for the progress bar
  /// * `message` - Optional initial message
  pub fn new_optional(
    total: u64,
    show_progress: bool,
    style: ProgressStyle,
    message: Option<&str>,
  ) -> Option<Arc<ProgressBar>> {
    if !show_progress {
      return None;
    }

    let pb = ProgressBar::new(total);
    pb.set_style(
      indicatif::ProgressStyle::default_bar()
        .template(style.template())
        .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
        .progress_chars(style.progress_chars()),
    );

    if let Some(msg) = message {
      pb.set_message(msg.to_string());
    }

    Some(Arc::new(pb))
  }

  /// Create a new progress bar (always creates one).
  pub fn new(total: u64, style: ProgressStyle, message: Option<&str>) -> Arc<ProgressBar> {
    Self::new_optional(total, true, style, message).expect("Progress bar should be created")
  }

  /// Increment the progress bar if it exists.
  pub fn increment(progress: &Option<Arc<ProgressBar>>) {
    if let Some(pb) = progress {
      pb.inc(1);
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

  /// Finish and clear the progress bar if it exists.
  pub fn finish_and_clear(progress: &Option<Arc<ProgressBar>>) {
    if let Some(pb) = progress {
      pb.finish_and_clear();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_optional_enabled() {
    let progress = ProgressManager::new_optional(100, true, ProgressStyle::Standard, Some("Test"));
    assert!(progress.is_some());
  }

  #[test]
  fn test_new_optional_disabled() {
    let progress = ProgressManager::new_optional(100, false, ProgressStyle::Standard, Some("Test"));
    assert!(progress.is_none());
  }

  #[test]
  fn test_new_progress_bar() {
    let progress = ProgressManager::new(100, ProgressStyle::Standard, Some("Test"));
    assert_eq!(progress.length(), Some(100));
  }

  #[test]
  fn test_increment() {
    let progress = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::increment(&progress);
    if let Some(pb) = &progress {
      assert_eq!(pb.position(), 1);
    }
  }

  #[test]
  fn test_increment_none() {
    let progress: Option<Arc<ProgressBar>> = None;
    ProgressManager::increment(&progress); // Should not panic
  }

  #[test]
  fn test_set_message() {
    let progress = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::set_message(&progress, "New message");
    // Message is set (no easy way to verify, but it shouldn't panic)
  }

  #[test]
  fn test_finish() {
    let progress = ProgressManager::new_optional(100, true, ProgressStyle::Standard, None);
    ProgressManager::finish(&progress, "Done");
    if let Some(pb) = &progress {
      assert!(pb.is_finished());
    }
  }

  #[test]
  fn test_progress_style_templates() {
    // Just verify templates are valid strings
    assert!(!ProgressStyle::Standard.template().is_empty());
    assert!(!ProgressStyle::Compact.template().is_empty());
    assert!(!ProgressStyle::Detailed.template().is_empty());
    assert!(!ProgressStyle::Spinner.template().is_empty());
  }
}