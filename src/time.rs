//! Time types for the Link clock domain.
//!
//! This module provides [`Instant`] and [`Duration`] types that are specific
//! to the Link clock, which is synchronized across all connected peers.

use std::{
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    time::Duration as StdDuration,
};

/// A point in time on the Link clock, measured in microseconds.
///
/// `Instant` represents an absolute timestamp from Link's internal clock,
/// which is synchronized across all connected peers. It is analogous to
/// [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html) but specific to the Link clock domain.
///
/// # Creating `Instant` values
///
/// You typically obtain an `Instant` from [`Link::clock_now`](crate::Link::clock_now) or
/// [`SessionState::transport_state_time`](crate::SessionState::transport_state_time):
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
///
/// let link = Link::new(120.0).unwrap();
/// let now = link.clock_now();
/// ```
///
/// # Arithmetic
///
/// `Instant` supports addition and subtraction with [`Duration`]:
///
/// ```no_run
/// use esp_idf_ableton_link::{Link, Duration};
///
/// let link = Link::new(120.0).unwrap();
/// let now = link.clock_now();
/// let later = now + Duration::from_millis(100);
/// let earlier = now - Duration::from_millis(50);
/// ```
///
/// Subtracting two `Instant` values yields a [`Duration`]:
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
///
/// let link = Link::new(120.0).unwrap();
/// let t1 = link.clock_now();
/// // ... some time passes ...
/// let t2 = link.clock_now();
/// let elapsed = t2 - t1; // Duration
/// ```
///
/// For convenience, [`std::time::Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html) is also supported:
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
/// use std::time::Duration;
///
/// let link = Link::new(120.0).unwrap();
/// let now = link.clock_now();
/// let later = now + Duration::from_millis(100);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(i64);

impl Instant {
    /// Create an `Instant` from microseconds.
    #[must_use]
    pub(crate) const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Get the time value as microseconds (signed).
    #[must_use]
    pub(crate) const fn as_micros(self) -> i64 {
        self.0
    }

    /// Get the time value as an unsigned 64-bit integer (microseconds).
    ///
    /// This performs a bit-preserving cast. Link's clock is based on
    /// `steady_clock` which always returns non-negative values, so this
    /// is safe for normal use.
    #[must_use]
    pub(crate) const fn as_u64(self) -> u64 {
        self.0.cast_unsigned()
    }

    /// Add microseconds to this time.
    #[must_use]
    pub const fn add_micros(self, micros: i64) -> Self {
        Self(self.0 + micros)
    }

    /// Subtract microseconds from this time.
    #[must_use]
    pub const fn sub_micros(self, micros: i64) -> Self {
        Self(self.0 - micros)
    }

    /// Add milliseconds to this time.
    #[must_use]
    pub const fn add_millis(self, millis: i64) -> Self {
        Self(self.0 + millis * 1_000)
    }

    /// Subtract milliseconds from this time.
    #[must_use]
    pub const fn sub_millis(self, millis: i64) -> Self {
        Self(self.0 - millis * 1_000)
    }

    /// Add seconds to this time.
    #[must_use]
    pub const fn add_secs(self, secs: i64) -> Self {
        Self(self.0 + secs * 1_000_000)
    }

    /// Subtract seconds from this time.
    #[must_use]
    pub const fn sub_secs(self, secs: i64) -> Self {
        Self(self.0 - secs * 1_000_000)
    }
}

/// A duration of time in microseconds, for use with [`Instant`].
///
/// `Duration` is a lightweight alternative to [`std::time::Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html) that
/// avoids the overhead of nanosecond precision and `u128` arithmetic on
/// embedded systems. Unlike `std::time::Duration`, this type supports
/// **signed** values, allowing representation of negative durations.
///
/// # Creating `Duration` values
///
/// ```no_run
/// use esp_idf_ableton_link::Duration;
///
/// let d1 = Duration::from_micros(500);
/// let d2 = Duration::from_millis(10);
/// let d3 = Duration::from_secs(1);
/// ```
///
/// # Arithmetic
///
/// `Duration` supports multiplication and division by `i64`:
///
/// ```no_run
/// use esp_idf_ableton_link::Duration;
///
/// let d = Duration::from_millis(100);
/// let doubled = d * 2;
/// let halved = d / 2;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(i64);

impl Duration {
    /// A duration of zero.
    pub const ZERO: Self = Self(0);

    /// Create a `Duration` from microseconds.
    #[must_use]
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Create a `Duration` from milliseconds.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis * 1_000)
    }

    /// Create a `Duration` from seconds.
    #[must_use]
    pub const fn from_secs(secs: i64) -> Self {
        Self(secs * 1_000_000)
    }

    /// Get the duration as microseconds.
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.0
    }

    /// Get the duration as milliseconds (truncating).
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.0 / 1_000
    }

    /// Get the duration as seconds (truncating).
    #[must_use]
    pub const fn as_secs(self) -> i64 {
        self.0 / 1_000_000
    }

    /// Returns the absolute value of this duration.
    #[must_use]
    pub const fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i64> for Duration {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl MulAssign<i64> for Duration {
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}

impl Div<i64> for Duration {
    type Output = Self;

    fn div(self, rhs: i64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<i64> for Duration {
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl Add<StdDuration> for Instant {
    type Output = Self;

    fn add(self, rhs: StdDuration) -> Self::Output {
        let micros = i64::try_from(rhs.as_micros()).unwrap_or(i64::MAX);
        Self(self.0 + micros)
    }
}

impl AddAssign<StdDuration> for Instant {
    fn add_assign(&mut self, rhs: StdDuration) {
        *self = *self + rhs;
    }
}

impl Sub<StdDuration> for Instant {
    type Output = Self;

    fn sub(self, rhs: StdDuration) -> Self::Output {
        let micros = i64::try_from(rhs.as_micros()).unwrap_or(i64::MAX);
        Self(self.0 - micros)
    }
}

impl SubAssign<StdDuration> for Instant {
    fn sub_assign(&mut self, rhs: StdDuration) {
        *self = *self - rhs;
    }
}
