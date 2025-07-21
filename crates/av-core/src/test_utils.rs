/// Default tolerance for floating-point comparisons
pub const DEFAULT_TOLERANCE: f64 = 1e-10;

/// Assert that two floating-point numbers are approximately equal
pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
  assert!(
    (actual - expected).abs() < tolerance,
    "Values not approximately equal:\n  actual:   {}\n  expected: {}\n  diff:     {}\n  tolerance: {}",
    actual,
    expected,
    (actual - expected).abs(),
    tolerance
  );
}

/// Assert that two floating-point percentages are approximately equal
pub fn assert_percentage_eq(actual: f64, expected: f64) {
  // Use a slightly larger tolerance for percentages
  assert_approx_eq(actual, expected, 1e-8);
}

/// Assert that a floating-point value is approximately zero
pub fn assert_approx_zero(value: f64) {
  assert_approx_eq(value, 0.0, DEFAULT_TOLERANCE);
}
