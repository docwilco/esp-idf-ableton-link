//! Safe Rust wrapper for Ableton Link on ESP32 via ESP-IDF.
//!
//! This crate provides a safe Rust API for [Ableton
//! Link](https://www.ableton.com/en/link/), enabling musical applications to
//! synchronize tempo and beat phase over a local network on ESP32 hardware.
//!
//! # Supported Hardware
//!
//! Currently only Xtensa-based ESP32 chips are supported:
//!
//! - ESP32
//! - ESP32-S2
//! - ESP32-S3
//!
//! # Usage
//!
//! It is recommended to start your project using the
//! [esp-idf-template](https://github.com/esp-rs/esp-idf-template?tab=readme-ov-file#generate-the-project).
//!
//! ## Required Configuration
//!
//! This crate requires specific configuration in your ESP32 project. All four
//! steps below are mandatory.
//!
//! ### 1. Add the crate and build dependency
//!
//! ```sh
//! cargo add esp-idf-ableton-link
//! cargo add --build embuild
//! ```
//!
//! ### 2. Set `ESP_IDF_SYS_ROOT_CRATE` in `.cargo/config.toml`
//!
//! The `ESP_IDF_SYS_ROOT_CRATE` environment variable must be set so that
//! `esp-idf-sys` can discover the extra component configuration from this
//! crate.
//!
//! Add to your project's `.cargo/config.toml`:
//!
//! ```toml
//! [build]
//! target = "xtensa-esp32-espidf"
//!
//! [target.xtensa-esp32-espidf]
//! linker = "ldproxy"
//! runner = "espflash flash --monitor"
//! rustflags = [ "--cfg",  "espidf_time64"]
//!
//! [env]
//! MCU="esp32"
//! ESP_IDF_VERSION = "v5.3.3"
//! ESP_IDF_SYS_ROOT_CRATE = "your-firmware-crate-name"
//! ```
//!
//! Replace `your-firmware-crate-name` with the `name` field from your
//! project's `Cargo.toml`.
//!
//! For more information, see the [ESP-IDF configuration
//! documentation](https://github.com/esp-rs/esp-idf-sys/blob/master/BUILD-OPTIONS.md#esp-idf-configuration).
//!
//! ### 3. Enable C++ Exceptions in `sdkconfig.defaults`
//!
//! Ableton Link requires C++ exception support. Add this to your project's
//! `sdkconfig.defaults`:
//!
//! ```text
//! CONFIG_COMPILER_CXX_EXCEPTIONS=y
//! ```
//!
//! You may also want to increase the main task stack size (Rust often needs
//! more than the default 3KB):
//!
//! ```text
//! CONFIG_ESP_MAIN_TASK_STACK_SIZE=8000
//! ```
//!
//! For more information about ESP-IDF sdkconfig options, see the [ESP-IDF
//! KConfig
//! reference](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/kconfig.html).
//!
//! ### 4. Add `embuild::espidf::sysenv::output()` to your `build.rs`
//!
//! Your project's `build.rs` must call `embuild::espidf::sysenv::output()` to
//! propagate ESP-IDF configuration flags (such as
//! `esp_idf_compiler_cxx_exceptions`) to the Rust compiler:
//!
//! ```ignore
//! fn main() {
//!     embuild::espidf::sysenv::output();
//! }
//! ```
//!
//! # Example
//!
//! ```ignore
//! use esp_idf_ableton_link::Link;
//!
//! // Create a new Link instance with 120 BPM
//! let mut link = Link::new(120.0).expect("Failed to create Link");
//!
//! // Enable Link to start synchronizing
//! link.enable();
//!
//! // Wait for Link to discover and sync with any existing session
//! esp_idf_svc::hal::delay::FreeRtos::delay_ms(4000);
//!
//! // Capture session state and read tempo
//! let state = link.capture_app_session_state().unwrap();
//! let tempo = state.tempo();
//! log::info!("Current tempo: {} BPM", tempo);
//!
//! // Get the current beat position
//! let now = link.clock_now();
//! let beat = state.beat_at_time(now, 4.0); // 4 beats per bar
//! log::info!("Current beat: {}", beat);
//! ```
//!
//! # Application vs Audio Thread Session State
//!
//! Link provides two sets of session state functions for different contexts:
//!
//! - **Application thread functions** ([`Link::capture_app_session_state`],
//!   [`Link::commit_app_session_state`]): For use from general application
//!   code. These may briefly block to synchronize with the audio thread.
//!   They are thread-safe and can be called from multiple threads/tasks.
//!
//! - **Audio thread functions** ([`AudioLink::capture_session_state`],
//!   [`AudioLink::commit_session_state`]): For use from realtime audio
//!   callbacks. These are lock-free and will never block, making them safe
//!   for low-latency audio processing.
//!
//! If your application has a dedicated audio thread with realtime constraints,
//! use [`Link::bind_audio_thread`] to obtain an [`AudioLink`] handle and use
//! its methods exclusively from that thread. For simpler applications without
//! strict realtime requirements, the application thread functions are
//! sufficient.
//!
//! The Link library recommends avoiding concurrent session state modifications
//! from both application and audio threads. This crate enforces that
//! recommendation: the [`AudioLink`] handle mutably borrows the [`Link`]
//! instance, preventing concurrent access at compile time.
//!
//! # The Timeline
//!
//! Link maintains a **timeline**—a mapping from wall-clock time to beat values.
//! This timeline is synchronized across all peers in a session, enabling
//! applications to know "where we are in the music" at any given moment.
//!
//! ## Tempo
//!
//! The timeline progresses at a rate determined by the **tempo** (beats per
//! minute). At 120 BPM, beat values increase by 2.0 per second. Tempo changes
//! are propagated to all peers.
//!
//! ## Beats
//!
//! Beat values are continuous `f64` numbers. The integer part is the number of
//! completed beats; the fractional part is the position within the current beat.
//! For example, `4.75` means 4 beats completed plus three-quarters through the
//! 5th beat.
//!
//! Each peer has its own beat origin, so absolute beat values differ between
//! peers. What's synchronized is the **phase** (see below).
//!
//! ## Quantum and Phase
//!
//! - **Quantum**: The number of beats per musical cycle (typically a bar).
//!   Common values:
//!   - `4.0` — 4/4 time (most electronic music, rock, pop)
//!   - `3.0` — 3/4 time (waltz, some ballads)
//!   - `6.0` — 6/8 time (jigs, some ballads)
//!
//! - **Phase**: Position within the current cycle, in range `[0, quantum)`.
//!   Conceptually `beat % quantum` (but handles negatives correctly).
//!   For example, with quantum `4.0`:
//!   - Beat `0.0` → phase `0.0` (downbeat)
//!   - Beat `1.5` → phase `1.5` (halfway through beat 2)
//!   - Beat `4.0` → phase `0.0` (next downbeat)
//!   - Beat `7.25` → phase `3.25` (one quarter through beat 4)
//!   - Beat `-1.0` → phase `3.0` (one beat before the downbeat)
//!
//! Quantum is a **local parameter**—each peer chooses its own. Peers using the
//! same quantum will have synchronized phases (downbeats align). Peers using
//! different quantums only align at beats divisible by both.
//!
//! For example, if Peer A uses quantum `4.0` and Peer B uses quantum `3.0`:
//! - At beat 12.0: both are at phase 0.0 (downbeat) ✓
//! - At beat 15.0: A is at phase 3.0, B is at phase 0.0 (downbeat) ✗
//! - At beat 16.0: A is at phase 0.0 (downbeat), B is at phase 1.0 ✗
//! - At beat 24.0: both are at phase 0.0 (downbeat) ✓
//!
//! Their downbeats only align at multiples of 12 (the LCM of 3 and 4).
//!
//! ## Beat Magnitude vs Phase
//!
//! Each Link peer maintains its own beat timeline with an arbitrary origin.
//! The absolute beat value (magnitude) will differ between peers, but the
//! **phase** (position within the cycle) is synchronized across all peers.
//!
//! For example, with quantum `4.0`:
//! - Peer A might be at beat `12.5` (phase `0.5`)
//! - Peer B might be at beat `100.5` (phase `0.5`)
//! - Peer C might be at beat `0.5` (phase `0.5`)
//!
//! All three are synchronized: they're all halfway through the first beat of
//! their respective bars.
//!
//! ## Shifting the Timeline
//!
//! When you "request a beat at a time" via [`SessionState::request_beat_at_time`],
//! you're shifting the entire timeline so that beat aligns with that time. This
//! affects beat values at all times (past, present, future). If you map beat
//! `0.0` to a time 2 beats in the future, the current beat becomes `-2.0`.
//!
//! ## Practical Implications
//!
//! - Use [`SessionState::phase_at_time`] when you need to know where you are in
//!   a bar/cycle (e.g., for triggering events on the downbeat).
//! - Use [`SessionState::beat_at_time`] when you need to track continuous
//!   progress or count beats locally.
//! - Use [`SessionState::request_beat_at_time`] to adjust your local beat value
//!   while preserving phase alignment with the session.
//!
//! ## Examples
//!
//! ### Triggering on the Downbeat
//!
//! To find the exact time of the next downbeat and schedule an action:
//!
//! ```ignore
//! let quantum = 4.0; // 4/4 time
//! let state = link.capture_app_session_state().unwrap();
//!
//! // Find the next downbeat after the current time
//! let now = link.clock_now();
//! let current_beat = state.beat_at_time(now, quantum);
//! let current_phase = state.phase_at_time(now, quantum);
//! let next_downbeat_beat = current_beat + (quantum - current_phase);
//! let next_downbeat_time = state.time_at_beat(next_downbeat_beat, quantum);
//!
//! // Compute delay for use with ESP-IDF timer APIs
//! let delay_us = (next_downbeat_time - now).as_micros();
//! // esp_timer_start_once(timer_handle, delay_us as u64);
//! ```
//!
//! ### Counting Beats Locally
//!
//! To count how many beats have elapsed since your app started:
//!
//! ```ignore
//! let start_time = link.clock_now();
//! let state = link.capture_app_session_state().unwrap();
//! let start_beat = state.beat_at_time(start_time, quantum);
//!
//! // Later...
//! let now = link.clock_now();
//! let state = link.capture_app_session_state().unwrap();
//! let current_beat = state.beat_at_time(now, quantum);
//! let beats_elapsed = current_beat - start_beat;
//! ```
//!
//! ### Aligning to a Specific Beat
//!
//! To make your local beat counter start at 0 on the next downbeat:
//!
//! ```ignore
//! let quantum = 4.0;
//! let now = link.clock_now();
//! let mut state = link.capture_app_session_state().unwrap();
//!
//! // Request that beat 0.0 occurs at the current time.
//! // - If alone: beat 0.0 is mapped to `now` immediately.
//! // - If with peers: beat 0.0 is mapped to the next time the session
//! //   phase is 0.0 (the next downbeat), preserving sync.
//! state.request_beat_at_time(0.0, now, quantum);
//! link.commit_app_session_state(&state);
//! ```
//!
//! # Transport State
//!
//! Separately from the timeline, Link can synchronize **transport state**
//! (play/stop). This is opt-in via [`Link::enable_transport_sync`].
//!
//! ## The Transport State Model
//!
//! Transport state consists of two pieces:
//! - A [`TransportState`] ([`Play`](TransportState::Play) or [`Stop`](TransportState::Stop))
//! - A timestamp indicating when that state took/takes effect
//!
//! The state may be **currently active** (timestamp in the past) or **scheduled**
//! (timestamp in the future). Use [`SessionState::transport_state`] to get the
//! state and [`SessionState::transport_state_time`] to determine when:
//!
//! ```ignore
//! let state = session.transport_state();      // Play or Stop
//! let when = session.transport_state_time();  // When it took/takes effect
//! let now = link.clock_now();
//!
//! if when < now {
//!     // State is currently active
//! } else {
//!     // State is scheduled for the future
//! }
//! ```
//!
//! ## Transport State is Independent of the Timeline
//!
//! Transport state does **not** affect the timeline. The beat/time mapping
//! continues unchanged regardless of whether transport is playing or stopped.
//! It's up to your application to decide what "playing" and "stopped" mean
//! (e.g., producing sound or not).
//!
//! ## When Transport Sync is Disabled
//!
//! Even with transport sync disabled ([`Link::disable_transport_sync`]), you
//! can still use the transport state API locally:
//!
//! - [`SessionState::set_transport_state_at`] and related methods work normally
//! - [`SessionState::transport_state`] and [`SessionState::transport_state_time`]
//!   reflect your local changes after committing
//!
//! The difference is that your transport state changes won't be broadcast to
//! peers, and you won't receive transport state changes from peers. This can
//! be useful for tracking play/stop state locally without participating in
//! session-wide transport sync.
//!
//! # Naming Differences from the C/C++ API
//!
//! This crate uses Rust-idiomatic naming that differs from the original C API
//! (`abl_link.h`) and C++ API (`Link.hpp`). Key differences:
//!
//! | This crate | C API | C++ API | Notes |
//! |------------|-------|---------|-------|
//! | [`TransportState`] | `bool` | `bool` | We use an enum with [`Play`](TransportState::Play)/[`Stop`](TransportState::Stop) variants |
//! | [`transport_state`](SessionState::transport_state) | `abl_link_is_playing` | `isPlaying` | Returns [`TransportState`] enum |
//! | [`set_transport_state_at`](SessionState::set_transport_state_at) | `abl_link_set_is_playing` | `setIsPlaying` | Takes [`TransportState`] enum |
//! | [`transport_state_time`](SessionState::transport_state_time) | `abl_link_time_for_is_playing` | `timeForIsPlaying` | Original name is confusing |
//! | [`enable_transport_sync`](Link::enable_transport_sync) | `abl_link_enable_start_stop_sync` | `enableStartStopSync` | "Transport" is more descriptive |
//! | [`is_transport_sync_enabled`](Link::is_transport_sync_enabled) | `abl_link_is_start_stop_sync_enabled` | `isStartStopSyncEnabled` | |
//! | [`set_transport_state_callback`](Link::set_transport_state_callback) | `abl_link_set_start_stop_callback` | `setStartStopCallback` | |
//! | [`Instant`] | `int64_t` / `uint64_t` | `std::chrono::microseconds` | Newtype for type safety and clarity |
//! | [`clock_now`](Link::clock_now) | `abl_link_clock_micros` | `clock().micros()` | Returns [`Instant`] |
//!
//! The C API uses "is playing" terminology because transport state is
//! represented as a boolean. We chose [`TransportState`] with explicit
//! [`Play`](TransportState::Play)/[`Stop`](TransportState::Stop) variants for
//! clarity, since the state can be either currently active or scheduled for the
//! future (see [The Transport State Model](#the-transport-state-model)).

use std::{
    ffi::c_void,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    sync::Mutex,
    time::Duration as StdDuration,
};

use delegate::delegate;

type Callback<T> = Mutex<Option<Box<dyn FnMut(T) + Send>>>;

/// The transport state: [`Play`](Self::Play) or [`Stop`](Self::Stop).
///
/// Transport state represents the *target* state, which may already be active
/// or scheduled for the future. See the [Transport State](crate#transport-state)
/// section in the module documentation for details on interpreting transport
/// state with [`SessionState::transport_state_time`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportState {
    /// Transport is playing or scheduled to play.
    Play,
    /// Transport is stopped or scheduled to stop.
    Stop,
}

impl From<bool> for TransportState {
    fn from(playing: bool) -> Self {
        if playing {
            Self::Play
        } else {
            Self::Stop
        }
    }
}

impl From<TransportState> for bool {
    fn from(state: TransportState) -> Self {
        matches!(state, TransportState::Play)
    }
}

/// A point in time on the Link clock, measured in microseconds.
///
/// `Instant` represents an absolute timestamp from Link's internal clock,
/// which is synchronized across all connected peers. It is analogous to
/// [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html) but specific to the Link clock domain.
///
/// # Creating `Instant` values
///
/// You typically obtain an `Instant` from [`Link::clock_now`] or
/// [`SessionState::transport_state_time`]:
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

mod sys {
    // Allow wildcard imports for the sys module since there is nothing else in
    // this module.
    #[allow(clippy::wildcard_imports)]
    pub use esp_idf_sys::abl_link::*;
}

/// Error type for Link operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Failed to allocate memory.
    AllocationFailed,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "Failed to allocate memory"),
        }
    }
}

impl std::error::Error for LinkError {}

// Generic trampoline function for C callbacks.
extern "C" fn trampoline<T>(value: T, context: *mut c_void) {
    // Catch panics to prevent unwinding across FFI boundary
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Safety: context is a pointer to Mutex<Option<Box<dyn FnMut(T) + Send>>>
        // stored in the Link struct, which outlives all callbacks.
        let mutex = unsafe { &*context.cast::<Mutex<Option<Box<dyn FnMut(T) + Send>>>>() };
        if let Ok(mut guard) = mutex.lock()
            && let Some(callback) = guard.as_mut()
        {
            callback(value);
        }
    }));
}

/// A safe wrapper around an Ableton Link instance.
///
/// Link enables musical applications to synchronize tempo and beat phase over a
/// local network. Multiple Link-enabled applications can play in time without
/// requiring any configuration.
///
/// # Thread Safety
///
/// `Link` is both `Send` and `Sync`. All underlying C++ methods use internal
/// synchronization. For realtime audio contexts, use
/// [`bind_audio_thread`](Self::bind_audio_thread) to obtain an [`AudioLink`]
/// handle with non-blocking methods.
///
/// # Example
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
///
/// let mut link = Link::new(120.0).expect("Failed to create Link");
/// link.enable();
///
/// // Capture session state for reading/modifying
/// let mut state = link.capture_app_session_state().unwrap();
/// let now = link.clock_now();
/// let beat = state.beat_at_time(now, 4.0);
/// let phase = state.phase_at_time(now, 4.0);
///
/// // Modify and commit changes
/// state.set_tempo(140.0, now);
/// link.commit_app_session_state(&state);
/// ```
pub struct Link {
    handle: sys::abl_link,
    // Callbacks protected by mutex. The trampoline locks before calling,
    // ensuring callbacks cannot be dropped while executing.
    num_peers_callback: Callback<u64>,
    tempo_callback: Callback<f64>,
    start_stop_callback: Callback<bool>,
}

// Safety: Link holds a pointer to a heap-allocated C++ object. All methods
// we call are documented as "Thread-safe: yes" in abl_link.h, and callback
// context management is protected by a Mutex.
unsafe impl Send for Link {}
unsafe impl Sync for Link {}

impl Link {
    /// Create a new Link instance with the specified initial tempo.
    ///
    /// # Arguments
    ///
    /// * `initial_bpm` - The initial tempo in beats per minute (BPM). Typical
    ///   values range from 20.0 to 999.0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Link)` on success, or `Err(LinkError::AllocationFailed)` if
    /// memory allocation fails.
    ///
    /// # Errors
    ///
    /// Returns [`LinkError::AllocationFailed`] if the underlying C++ Link
    /// instance could not be allocated (typically due to memory exhaustion).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).expect("Failed to create Link");
    /// ```
    pub fn new(initial_bpm: f64) -> Result<Self, LinkError> {
        // Safety: abl_link_create is safe to call with any f64 value. It
        // allocates a new Link instance and returns a struct with impl pointer.
        let handle = unsafe { sys::abl_link_create(initial_bpm) };

        if handle.impl_.is_null() {
            Err(LinkError::AllocationFailed)
        } else {
            log::debug!(
                "Created Link instance at {:p} with {initial_bpm} BPM",
                handle.impl_
            );
            Ok(Self {
                handle,
                num_peers_callback: Mutex::new(None),
                tempo_callback: Mutex::new(None),
                start_stop_callback: Mutex::new(None),
            })
        }
    }

    /// Enable Link synchronization.
    ///
    /// When enabled, Link will discover and synchronize with other Link-enabled
    /// applications on the local network.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// link.enable();  // Start synchronizing
    /// // ... later ...
    /// link.disable(); // Stop synchronizing
    /// ```
    pub fn enable(&self) {
        self.set_enabled(true);
    }

    /// Disable Link synchronization.
    ///
    /// See also [`enable`](Self::enable).
    pub fn disable(&self) {
        self.set_enabled(false);
    }

    /// Enable or disable Link synchronization.
    ///
    /// This is useful when the enabled state comes from a variable.
    /// For static enable/disable, prefer [`enable`](Self::enable) and
    /// [`disable`](Self::disable) for readability.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable, `false` to disable.
    pub fn set_enabled(&self, enabled: bool) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_enable(self.handle, enabled) }
        log::debug!("Link {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if Link is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if Link is enabled and synchronizing, `false` otherwise.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_is_enabled(self.handle) }
    }

    /// Get the number of peers currently connected in the Link session.
    ///
    /// # Returns
    ///
    /// The number of other Link-enabled applications connected to this session.
    /// Returns 0 if no peers are connected (solo mode).
    #[must_use]
    pub fn num_peers(&self) -> u64 {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_num_peers(self.handle) }
    }

    /// Check if transport synchronization is enabled.
    ///
    /// When enabled, transport start/stop state is shared with other peers
    /// in the session. See [Transport State](crate#transport-state) for
    /// more details.
    ///
    /// # Returns
    ///
    /// `true` if transport sync is enabled, `false` otherwise.
    #[must_use]
    pub fn is_transport_sync_enabled(&self) -> bool {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_is_start_stop_sync_enabled(self.handle) }
    }

    /// Enable transport synchronization.
    ///
    /// When enabled, transport start/stop state is shared with other peers
    /// in the session. This allows multiple applications to start and stop
    /// playback together. See [Transport State](crate#transport-state) for
    /// more details.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// link.enable_transport_sync();
    /// ```
    pub fn enable_transport_sync(&self) {
        self.set_transport_sync_enabled(true);
    }

    /// Disable transport synchronization.
    ///
    /// See also [`enable_transport_sync`](Self::enable_transport_sync).
    pub fn disable_transport_sync(&self) {
        self.set_transport_sync_enabled(false);
    }

    /// Enable or disable transport synchronization.
    ///
    /// This is useful when the enabled state comes from a variable.
    /// For static enable/disable, prefer
    /// [`enable_transport_sync`](Self::enable_transport_sync) and
    /// [`disable_transport_sync`](Self::disable_transport_sync) for
    /// readability.
    ///
    /// See [Transport State](crate#transport-state) for more details.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable transport sync, `false` to disable.
    pub fn set_transport_sync_enabled(&self, enabled: bool) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_enable_start_stop_sync(self.handle, enabled) }
    }

    /// Capture the current Link session state from an application thread.
    ///
    /// The returned [`SessionState`] is a snapshot of the current Link state.
    /// It should be used in a local scope and not stored for later use, as it
    /// will become stale.
    ///
    /// To apply changes made to the session state, call
    /// [`commit_app_session_state`](Self::commit_app_session_state).
    ///
    /// # Errors
    ///
    /// Returns [`LinkError::AllocationFailed`] if the session state
    /// could not be allocated.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// let state = link.capture_app_session_state().unwrap();
    /// let tempo = state.tempo();
    /// ```
    pub fn capture_app_session_state(&self) -> Result<SessionState, LinkError> {
        // Safety: abl_link_create_session_state allocates a new session state.
        let session_state = unsafe { sys::abl_link_create_session_state() };

        if session_state.impl_.is_null() {
            return Err(LinkError::AllocationFailed);
        }

        // Safety: Both handles are valid.
        unsafe { sys::abl_link_capture_app_session_state(self.handle, session_state) };

        Ok(SessionState {
            handle: session_state,
        })
    }

    /// Commit the given session state to the Link session from an application
    /// thread.
    ///
    /// The given session state will replace the current Link session state.
    /// Modifications will be communicated to other peers in the session.
    ///
    /// # Note
    ///
    /// This method takes `&mut self` to prevent calling it while an
    /// [`AudioLink`] handle exists. The Link library recommends against
    /// modifying session state from both audio and application threads
    /// concurrently.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// let mut state = link.capture_app_session_state().unwrap();
    /// let now = link.clock_now();
    /// state.set_tempo(140.0, now);
    /// link.commit_app_session_state(&state);
    /// ```
    pub fn commit_app_session_state(&mut self, state: &SessionState) {
        // Safety: both handles are valid.
        unsafe { sys::abl_link_commit_app_session_state(self.handle, state.handle) }
    }

    /// Register a callback to be notified when the number of peers changes.
    ///
    /// The callback is invoked on a Link-managed thread whenever the number of
    /// peers in the Link session changes.
    ///
    /// Setting a new callback replaces any previously registered callback.
    /// Use [`clear_num_peers_callback`](Self::clear_num_peers_callback) to
    /// unregister without setting a new one.
    ///
    /// # Arguments
    ///
    /// * `callback` - A closure that receives the new peer count.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This can happen if a
    /// previously registered callback panicked. Note that panics in callbacks
    /// are caught to prevent unwinding across the FFI boundary, but the mutex
    /// will still be poisoned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.set_num_peers_callback(|num_peers| {
    ///     log::info!("Peer count changed: {}", num_peers);
    /// });
    /// link.enable();
    /// ```
    pub fn set_num_peers_callback<F>(&self, callback: F)
    where
        F: FnMut(u64) + Send + 'static,
    {
        let boxed: Box<dyn FnMut(u64) + Send> = Box::new(callback);

        // Lock mutex and replace callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.num_peers_callback.lock().unwrap() = Some(boxed);

        // Safety: handle is valid (checked in new()), trampoline has correct
        // signature. Context pointer is stable for Link's lifetime.
        let context = std::ptr::from_ref(&self.num_peers_callback)
            .cast_mut()
            .cast::<c_void>();
        unsafe {
            sys::abl_link_set_num_peers_callback(self.handle, Some(trampoline::<u64>), context);
        }
    }

    /// Clear the num peers callback without setting a new one.
    ///
    /// After calling this, no callback will be invoked when the peer count changes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. See
    /// [`set_num_peers_callback`](Self::set_num_peers_callback) for details.
    pub fn clear_num_peers_callback(&self) {
        // Lock mutex and clear callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.num_peers_callback.lock().unwrap() = None;
    }

    /// Register a callback to be notified when the session tempo changes.
    ///
    /// The callback is invoked on a Link-managed thread whenever the tempo
    /// of the Link session changes.
    ///
    /// Setting a new callback replaces any previously registered callback.
    /// Use [`clear_tempo_callback`](Self::clear_tempo_callback) to unregister
    /// without setting a new one.
    ///
    /// # Arguments
    ///
    /// * `callback` - A closure that receives the new tempo in BPM.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This can happen if a
    /// previously registered callback panicked. Note that panics in callbacks
    /// are caught to prevent unwinding across the FFI boundary, but the mutex
    /// will still be poisoned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.set_tempo_callback(|tempo| {
    ///     log::info!("Tempo changed: {} BPM", tempo);
    /// });
    /// link.enable();
    /// ```
    pub fn set_tempo_callback<F>(&self, callback: F)
    where
        F: FnMut(f64) + Send + 'static,
    {
        let boxed: Box<dyn FnMut(f64) + Send> = Box::new(callback);

        // Lock mutex and replace callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.tempo_callback.lock().unwrap() = Some(boxed);

        // Safety: handle is valid (checked in new()), trampoline has correct
        // signature. Context pointer is stable for Link's lifetime.
        let context = std::ptr::from_ref(&self.tempo_callback)
            .cast_mut()
            .cast::<c_void>();
        unsafe {
            sys::abl_link_set_tempo_callback(self.handle, Some(trampoline::<f64>), context);
        }
    }

    /// Clear the tempo callback without setting a new one.
    ///
    /// After calling this, no callback will be invoked when the tempo changes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. See
    /// [`set_tempo_callback`](Self::set_tempo_callback) for details.
    pub fn clear_tempo_callback(&self) {
        // Lock mutex and clear callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.tempo_callback.lock().unwrap() = None;
    }

    /// Register a callback to be notified when the transport state changes.
    ///
    /// The callback is invoked on a Link-managed thread whenever the *intent*
    /// of the transport state changes. This happens immediately when a peer
    /// commits a new transport state, even if the state change is scheduled
    /// for the future.
    ///
    /// To determine *when* the transport state takes effect, capture the
    /// session state and call [`SessionState::transport_state_time`]. This
    /// enables scheduling actions to coincide with the actual start/stop time.
    ///
    /// Setting a new callback replaces any previously registered callback.
    /// Use [`clear_transport_state_callback`](Self::clear_transport_state_callback)
    /// to unregister without setting a new one.
    ///
    /// # Arguments
    ///
    /// * `callback` - A closure that receives the new [`TransportState`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. This can happen if a
    /// previously registered callback panicked. Note that panics in callbacks
    /// are caught to prevent unwinding across the FFI boundary, but the mutex
    /// will still be poisoned.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use esp_idf_ableton_link::{Link, TransportState};
    /// use std::sync::mpsc;
    ///
    /// let (tx, rx) = mpsc::channel();
    /// let mut link = Link::new(120.0).unwrap();
    /// link.enable_transport_sync();
    /// link.set_transport_state_callback(move |state| {
    ///     // Notify main thread of state change
    ///     let _ = tx.send(state);
    /// });
    /// link.enable();
    ///
    /// // In main loop, handle transport changes
    /// if let Ok(state) = rx.try_recv() {
    ///     let session = link.capture_app_session_state().unwrap();
    ///     let start_time = session.transport_state_time();
    ///     let now = link.clock_now();
    ///     
    ///     if state == TransportState::Play && start_time > now {
    ///         // Transport will start in the future - schedule action
    ///         let delay_us = (start_time - now).as_micros();
    ///         // esp_timer_start_once(timer_handle, delay_us as u64);
    ///     }
    /// }
    /// ```
    pub fn set_transport_state_callback<F>(&self, mut callback: F)
    where
        F: FnMut(TransportState) + Send + 'static,
    {
        // Wrap the user's callback to convert bool -> TransportState
        let wrapper = move |is_playing: bool| {
            callback(TransportState::from(is_playing));
        };
        let boxed: Box<dyn FnMut(bool) + Send> = Box::new(wrapper);

        // Lock mutex and replace callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.start_stop_callback.lock().unwrap() = Some(boxed);

        // Safety: handle is valid (checked in new()), trampoline has correct
        // signature. Context pointer is stable for Link's lifetime.
        let context = std::ptr::from_ref(&self.start_stop_callback)
            .cast_mut()
            .cast::<c_void>();
        unsafe {
            sys::abl_link_set_start_stop_callback(self.handle, Some(trampoline::<bool>), context);
        }
    }

    /// Clear the transport state callback without setting a new one.
    ///
    /// After calling this, no callback will be invoked when the transport state changes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. See
    /// [`set_transport_state_callback`](Self::set_transport_state_callback) for details.
    pub fn clear_transport_state_callback(&self) {
        // Lock mutex and clear callback. Any in-flight trampoline call
        // is blocked until we release the lock.
        *self.start_stop_callback.lock().unwrap() = None;
    }

    /// Get the current Link clock time.
    ///
    /// This returns the current time from Link's internal clock, which is
    /// synchronized across all connected peers. Use this value as input to
    /// [`SessionState::beat_at_time`] and [`SessionState::phase_at_time`].
    ///
    /// # Returns
    ///
    /// The current [`Instant`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// let now = link.clock_now();
    /// ```
    #[must_use]
    pub fn clock_now(&self) -> Instant {
        // Safety: handle is valid (checked in new()).
        Instant(unsafe { sys::abl_link_clock_micros(self.handle) })
    }

    /// Bind this Link instance for audio-thread access.
    ///
    /// Returns an [`AudioLink`] handle that provides realtime-safe
    /// (non-blocking) methods for capturing and committing session state.
    /// The handle is bound to the current thread and cannot be sent to other
    /// threads.
    ///
    /// While the `AudioLink` handle exists, this `Link` instance is mutably
    /// borrowed, preventing concurrent access to app-thread session state
    /// methods.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.enable();
    ///
    /// // Bind to the audio thread
    /// let audio_link = link.bind_audio_thread();
    ///
    /// // Use realtime-safe methods
    /// let state = audio_link.capture_session_state().unwrap();
    /// let now = audio_link.clock_now();
    /// let beat = state.beat_at_time(now, 4.0);
    /// ```
    pub fn bind_audio_thread(&mut self) -> AudioLink<'_> {
        AudioLink {
            link: self,
            _not_send_sync: PhantomData,
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        log::debug!("Destroying Link instance at {:p}", self.handle.impl_);

        // Safety: handle is valid (checked in new()). After this call, handle
        // is invalid but that's fine since we're being dropped.
        // Note: abl_link_destroy waits for pending callbacks to complete,
        // so the callback Mutexes will be dropped only after that.
        unsafe { sys::abl_link_destroy(self.handle) }
    }
}

/// A handle for accessing Link session state from a realtime audio thread.
///
/// This type provides realtime-safe (non-blocking) methods for capturing and
/// committing session state. It is designed for use in audio callbacks or
/// high-priority tasks where blocking is not acceptable.
///
/// `AudioLink` also exposes a subset of [`Link`] methods that are realtime-safe,
/// such as [`clock_now`](Self::clock_now), [`is_enabled`](Self::is_enabled),
/// and [`num_peers`](Self::num_peers). This allows audio code to access these
/// methods without needing a separate reference to the [`Link`] instance.
///
/// # Thread Safety
///
/// `AudioLink` is `!Send` and `!Sync`, meaning it cannot be sent to or shared
/// with other threads. This ensures that the audio session state functions are
/// only called from the thread that created the handle, as required by the
/// underlying Link library.
///
/// # Exclusivity
///
/// While an `AudioLink` handle exists, the parent [`Link`] instance is mutably
/// borrowed, preventing concurrent access to app-thread session state methods.
/// This matches the Link library's recommendation to avoid modifying session
/// state from both audio and application threads concurrently.
///
/// # Example
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
///
/// let mut link = Link::new(120.0).unwrap();
/// link.enable();
///
/// // In your audio thread setup:
/// let audio_link = link.bind_audio_thread();
///
/// // In the audio callback (same thread):
/// let state = audio_link.capture_session_state().unwrap();
/// let now = audio_link.clock_now();
/// let beat = state.beat_at_time(now, 4.0);
/// // ... generate audio based on beat position ...
/// ```
pub struct AudioLink<'a> {
    link: &'a mut Link,
    // PhantomData<*mut ()> makes this type !Send and !Sync
    _not_send_sync: PhantomData<*mut ()>,
}

impl AudioLink<'_> {
    delegate! {
        to self.link {
            /// See [`Link::clock_now`]. This method is realtime-safe.
            #[must_use]
            pub fn clock_now(&self) -> Instant;

            /// See [`Link::is_enabled`]. This method is realtime-safe.
            #[must_use]
            pub fn is_enabled(&self) -> bool;

            /// See [`Link::num_peers`]. This method is realtime-safe.
            #[must_use]
            pub fn num_peers(&self) -> u64;

            /// See [`Link::is_transport_sync_enabled`]. This method is realtime-safe.
            #[must_use]
            pub fn is_transport_sync_enabled(&self) -> bool;

            /// See [`Link::enable_transport_sync`]. This method is realtime-safe.
            pub fn enable_transport_sync(&self);

            /// See [`Link::disable_transport_sync`]. This method is realtime-safe.
            pub fn disable_transport_sync(&self);

            /// See [`Link::set_transport_sync_enabled`]. This method is realtime-safe.
            pub fn set_transport_sync_enabled(&self, enabled: bool);
        }
    }

    /// Capture the current Link session state (realtime-safe).
    ///
    /// This method is non-blocking and safe to call from a realtime audio
    /// context. The returned [`SessionState`] is a snapshot that should be
    /// used locally and not stored for later use.
    ///
    /// # Errors
    ///
    /// Returns [`LinkError::AllocationFailed`] if the session state
    /// could not be allocated.
    pub fn capture_session_state(&self) -> Result<SessionState, LinkError> {
        // Safety: abl_link_create_session_state allocates a new session state.
        let session_state = unsafe { sys::abl_link_create_session_state() };

        if session_state.impl_.is_null() {
            return Err(LinkError::AllocationFailed);
        }

        // Safety: Both handles are valid. AudioLink's !Send guarantee ensures
        // we're on the designated audio thread.
        unsafe { sys::abl_link_capture_audio_session_state(self.link.handle, session_state) };

        Ok(SessionState {
            handle: session_state,
        })
    }

    /// Commit the given session state to the Link session (realtime-safe).
    ///
    /// This method is non-blocking and safe to call from a realtime audio
    /// context. The given session state will replace the current Link session
    /// state, and modifications will be communicated to other peers.
    pub fn commit_session_state(&self, state: &SessionState) {
        // Safety: Both handles are valid. AudioLink's !Send guarantee ensures
        // we're on the designated audio thread.
        unsafe { sys::abl_link_commit_audio_session_state(self.link.handle, state.handle) }
    }
}

/// A snapshot of the Link session state.
///
/// This represents a point-in-time view of the Link session's timeline and
/// transport state. It provides methods to read and modify tempo, beat
/// position, and transport (play/stop) state.
///
/// See the module documentation for background on:
/// - [The Timeline](crate#the-timeline) — tempo, beats, phase, and quantum
/// - [Transport State](crate#transport-state) — play/stop synchronization
///
/// # Usage
///
/// 1. Capture a session state with [`Link::capture_app_session_state`]
///    or [`AudioLink::capture_session_state`]
/// 2. Read values using [`tempo`](Self::tempo),
///    [`beat_at_time`](Self::beat_at_time), [`transport_state`](Self::transport_state), etc.
/// 3. Optionally modify using [`set_tempo`](Self::set_tempo),
///    [`request_beat_at_time`](Self::request_beat_at_time),
///    [`set_transport_state_at`](Self::set_transport_state_at), etc.
/// 4. Commit changes with [`Link::commit_app_session_state`]
///    or [`AudioLink::commit_session_state`]
///
/// # Important
///
/// This is a snapshot and will become stale. Don't store it for later use.
/// Capture a fresh state when you need current values.
pub struct SessionState {
    handle: sys::abl_link_session_state,
}

// Safety: SessionState is an independent snapshot with no references to Link.
// It can be safely moved between threads.
//
// Note: Sync is intentionally NOT implemented. The underlying C API does not
// document thread-safety for concurrent reads of session state, and the design
// intent is for session state to be used in a local scope after capture.
unsafe impl Send for SessionState {}

impl SessionState {
    /// Get the tempo of the timeline in Beats Per Minute.
    ///
    /// This is a stable value appropriate for display to the user. Beat time
    /// progress may not match this tempo exactly due to clock drift
    /// compensation.
    #[must_use]
    pub fn tempo(&self) -> f64 {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_tempo(self.handle) }
    }

    /// Set the timeline tempo to the given BPM value.
    ///
    /// The `time` parameter serves as the pivot point for the tempo change:
    /// the beat value at this time is preserved, while beat values at all
    /// other times are recalculated according to the new tempo. The tempo
    /// change affects the entire timeline immediately upon commit, not
    /// "starting at" the given time.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `bpm` - The new tempo in beats per minute.
    /// * `time` - The pivot point for the tempo change. The beat value at this
    ///   time remains unchanged; beats at other times shift according to the
    ///   new tempo.
    pub fn set_tempo(&mut self, bpm: f64, time: Instant) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_set_tempo(self.handle, bpm, time.as_micros()) }
    }

    /// Get the beat value at the given time for the given quantum.
    ///
    /// The beat value's magnitude is unique to this Link instance, but its
    /// phase with respect to the quantum is shared among all session peers.
    ///
    /// # Arguments
    ///
    /// * `time` - The time (from [`Link::clock_now`]).
    /// * `quantum` - The quantum (beats per cycle/bar).
    #[must_use]
    pub fn beat_at_time(&self, time: Instant, quantum: f64) -> f64 {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_beat_at_time(self.handle, time.as_micros(), quantum) }
    }

    /// Get the phase (position within a cycle) at the given time.
    ///
    /// The result is in the interval `[0, quantum)`. This is equivalent to
    /// `beat_at_time(t, q) % q` for non-negative beat values, but handles
    /// negative values correctly.
    ///
    /// # Arguments
    ///
    /// * `time` - The time (from [`Link::clock_now`]).
    /// * `quantum` - The quantum (beats per cycle/bar).
    #[must_use]
    pub fn phase_at_time(&self, time: Instant, quantum: f64) -> f64 {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_phase_at_time(self.handle, time.as_micros(), quantum) }
    }

    /// Get the time at which the given beat occurs for the given quantum.
    ///
    /// This is the inverse of [`beat_at_time`](Self::beat_at_time), assuming
    /// constant tempo.
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat value.
    /// * `quantum` - The quantum (beats per cycle/bar).
    #[must_use]
    pub fn time_at_beat(&self, beat: f64, quantum: f64) -> Instant {
        // Safety: handle is valid (checked in new()).
        Instant(unsafe { sys::abl_link_time_at_beat(self.handle, beat, quantum) })
    }

    /// Request a beat/time mapping, respecting session phase when not alone
    /// in the session (quantized launch).
    ///
    /// This only changes the local beat/time mapping; it does not affect other
    /// peers' beat magnitudes.
    ///
    /// # Behavior
    ///
    /// - **When alone** (no other peers): The beat is mapped to `at_time`.
    ///   After committing, `beat_at_time(at_time, quantum) == beat`.
    ///
    /// - **When not alone**: To avoid disrupting the session, the beat is
    ///   mapped to the first time **≥ `at_time`** where the session phase
    ///   matches the phase of `beat`. This enables "quantized launch" where
    ///   events happen in-phase with the session.
    ///
    /// # When Does the Timeline Shift?
    ///
    /// The timeline shifts **immediately upon commit**, not at `at_time`. The
    /// `at_time` parameter specifies which point on the timeline should have
    /// the given beat value—the entire timeline shifts to satisfy this
    /// constraint. This means `beat_at_time()` will return different values
    /// for **all** times (past, present, and future) after committing.
    ///
    /// For example, if you map beat `0.0` to a time 2 beats in the future,
    /// the current beat becomes `-2.0`. Negative beats are valid and represent
    /// a "count-in" before beat zero.
    ///
    /// # Example
    ///
    /// With quantum `4.0`, if the session is currently at phase `2.5` and you
    /// request beat `0.0` (phase `0.0`) at the current time:
    /// - When alone: beat `0.0` is mapped to now immediately.
    /// - When not alone: beat `0.0` is mapped to the next downbeat (when
    ///   session phase reaches `0.0`), which is 1.5 beats in the future.
    ///   The current beat becomes `-1.5`.
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map (only affects local magnitude, not session phase).
    /// * `time` - The earliest time for the mapping (actual time may be later
    ///   when not alone in the session).
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    pub fn request_beat_at_time(&mut self, beat: f64, time: Instant, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_request_beat_at_time(self.handle, beat, time.as_micros(), quantum);
        }
    }

    /// Forcibly shift the session phase, affecting all peers.
    ///
    /// Unlike [`request_beat_at_time`](Self::request_beat_at_time), this does not
    /// wait for phase alignment when other peers are connected. It shifts the
    /// session phase to match the requested beat's phase at the given time.
    ///
    /// # Effect on Other Peers
    ///
    /// Other peers' beat magnitudes are adjusted by the phase shift amount to
    /// keep everyone synchronized at the new phase. For example, with quantum
    /// `4.0`:
    ///
    /// - You are at beat 9.0 (phase 1.0) and force beat 0.0 (phase 0.0) at `now`
    /// - Your local beat becomes 0.0, the session phase reference shifts by -1.0
    /// - A peer at beat 109.0 (phase 1.0) becomes beat 108.0 (phase 0.0)
    ///
    /// The peer's magnitude changed by -1.0 to match the new session phase.
    /// This causes a beat discontinuity—the peer's beat counter jumps.
    ///
    /// **Warning:** This is anti-social behavior. Only use this for bridging an
    /// external clock source into a Link session. Most applications should use
    /// [`request_beat_at_time`](Self::request_beat_at_time) instead.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map (determines the new session phase).
    /// * `time` - The time to map it to.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn force_beat_at_time(&mut self, beat: f64, time: Instant, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_force_beat_at_time(self.handle, beat, time.as_u64(), quantum);
        }
    }

    /// Get the current transport state.
    ///
    /// This is part of the transport sync feature. Enable it via
    /// [`Link::enable_transport_sync`] to share state with peers.
    ///
    /// The returned state indicates the *target* transport state, which may
    /// already be in effect or scheduled for the future. Use
    /// [`transport_state_time`](Self::transport_state_time) to determine when
    /// the state took/takes effect.
    ///
    /// # Returns
    ///
    /// - [`TransportState::Play`] if transport is playing or scheduled to play.
    /// - [`TransportState::Stop`] if transport is stopped or scheduled to stop.
    #[must_use]
    pub fn transport_state(&self) -> TransportState {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_is_playing(self.handle) }.into()
    }

    /// Start transport at the specified time.
    ///
    /// This is part of the transport sync feature. Enable it via
    /// [`Link::enable_transport_sync`] to share state with peers.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `time` - The time at which playback starts.
    pub fn start_transport_at(&mut self, time: Instant) {
        self.set_transport_state_at(TransportState::Play, time);
    }

    /// Stop transport at the specified time.
    ///
    /// This is part of the transport sync feature. Enable it via
    /// [`Link::enable_transport_sync`] to share state with peers.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `time` - The time at which playback stops.
    pub fn stop_transport_at(&mut self, time: Instant) {
        self.set_transport_state_at(TransportState::Stop, time);
    }

    /// Set the transport state at the specified time.
    ///
    /// This is part of the transport sync feature. Enable it via
    /// [`Link::enable_transport_sync`] to share state with peers. The change
    /// takes effect at the specified time.
    ///
    /// This is useful when the transport state comes from a variable.
    /// For static start/stop, prefer [`start_transport_at`](Self::start_transport_at)
    /// and [`stop_transport_at`](Self::stop_transport_at) for readability.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `state` - The desired transport state.
    /// * `time` - The time at which the change takes effect.
    pub fn set_transport_state_at(&mut self, state: TransportState, time: Instant) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_set_is_playing(self.handle, state.into(), time.as_u64()) }
    }

    /// Get the time associated with the current transport state.
    ///
    /// Use this in combination with [`transport_state`](Self::transport_state)
    /// to determine whether the transport state is currently active or scheduled:
    ///
    /// - **Time in the past** (< `clock_now()`): The state from `transport_state()`
    ///   is currently in effect.
    /// - **Time in the future** (> `clock_now()`): The state from `transport_state()`
    ///   is scheduled to take effect at this time.
    ///
    /// The meaning of this time also depends on whether the transport state has
    /// been modified in this session state snapshot:
    ///
    /// - **Before any local modifications**: This is the time at which the
    ///   current transport state (playing or stopped) took effect or is
    ///   scheduled to take effect.
    ///
    /// - **After calling [`set_transport_state_at`] (or [`start_transport_at`]/
    ///   [`stop_transport_at`])**: This returns the `at_time` you provided.
    ///
    /// If no transport state has ever been set, returns a time of 0.
    ///
    /// [`set_transport_state_at`]: Self::set_transport_state_at
    /// [`start_transport_at`]: Self::start_transport_at
    /// [`stop_transport_at`]: Self::stop_transport_at
    #[must_use]
    pub fn transport_state_time(&self) -> Instant {
        // Safety: handle is valid (checked in new()).
        Instant(unsafe { sys::abl_link_time_for_is_playing(self.handle) }.cast_signed())
    }

    /// Request to map the given beat to the transport state time.
    ///
    /// This calls [`request_beat_at_time`](Self::request_beat_at_time) with
    /// the time from [`transport_state_time`](Self::transport_state_time).
    ///
    /// This is useful for quantized launch scenarios where you want the beat
    /// at the transport start time to be a specific value (e.g., 0.0 for the
    /// beginning of a song).
    ///
    /// **Note:** This is a no-op if transport is stopped
    /// ([`transport_state`](Self::transport_state) returns [`TransportState::Stop`]).
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map to the transport state time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn request_beat_at_transport_state_time(&mut self, beat: f64, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_request_beat_at_start_playing_time(self.handle, beat, quantum) }
    }

    /// Start transport and request a beat mapping in one operation.
    ///
    /// This is equivalent to calling [`start_transport_at`] followed by
    /// [`request_beat_at_transport_state_time`]. It starts transport at the
    /// given time and maps the given beat to that time.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`] or [`AudioLink::commit_session_state`].
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map to the start time.
    /// * `time` - The time at which transport starts.
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// [`start_transport_at`]: Self::start_transport_at
    /// [`request_beat_at_transport_state_time`]: Self::request_beat_at_transport_state_time
    pub fn start_transport_and_request_beat_at(
        &mut self,
        beat: f64,
        time: Instant,
        quantum: f64,
    ) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_set_is_playing_and_request_beat_at_time(
                self.handle,
                true,
                time.as_u64(),
                beat,
                quantum,
            );
        }
    }
}

impl Drop for SessionState {
    fn drop(&mut self) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_destroy_session_state(self.handle) }
    }
}
