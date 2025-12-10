//! Safe Rust wrapper for Ableton Link on ESP32 via ESP-IDF.
//!
//! This crate provides a safe Rust API for [Ableton
//! Link](https://www.ableton.com/en/link/), enabling musical applications to
//! synchronize tempo and beat phase over a local network on ESP32 hardware.
//!
//! ## Supported Hardware
//!
//! Currently only Xtensa-based ESP32 chips are supported:
//!
//! - ESP32
//! - ESP32-S2
//! - ESP32-S3
//!
//! ## Usage
//!
//! It is recommended to start your project using the
//! [esp-idf-template](https://github.com/esp-rs/esp-idf-template?tab=readme-ov-file#generate-the-project).
//!
//! ### Required Configuration
//!
//! This crate requires specific configuration in your ESP32 project. All four
//! steps below are mandatory.
//!
//! #### 1. Add the crate and build dependency
//!
//! ```sh
//! cargo add esp-idf-ableton-link
//! cargo add --build embuild
//! ```
//!
//! #### 2. Set `ESP_IDF_SYS_ROOT_CRATE` in `.cargo/config.toml`
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
//! #### 3. Enable C++ Exceptions in `sdkconfig.defaults`
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
//! #### 4. Add `embuild::espidf::sysenv::output()` to your `build.rs`
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
//! ## Example
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
//! ## Application vs Audio Thread Session State
//!
//! Link provides two sets of session state functions for different contexts:
//!
//! - **Application thread functions** ([`Link::capture_app_session_state`],
//!   [`Link::commit_app_session_state`]): For use from general application
//!   code. These may briefly block to synchronize with the audio thread.
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

use std::{
    ffi::c_void,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    sync::Mutex,
    time::Duration,
};

use delegate::delegate;

type Callback<T> = Mutex<Option<Box<dyn FnMut(T) + Send>>>;

/// A point in time on the Link clock, measured in microseconds.
///
/// `LinkTime` represents an absolute timestamp from Link's internal clock,
/// which is synchronized across all connected peers. It is analogous to
/// [`std::time::Instant`] but specific to the Link clock domain.
///
/// # Creating `LinkTime` values
///
/// You typically obtain a `LinkTime` from [`Link::clock_now`] or
/// [`SessionState::time_for_is_playing`]:
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
/// `LinkTime` supports addition and subtraction with [`Duration`]:
///
/// ```no_run
/// use esp_idf_ableton_link::{Link, LinkTime};
/// use std::time::Duration;
///
/// let link = Link::new(120.0).unwrap();
/// let now = link.clock_now();
/// let later = now + Duration::from_millis(100);
/// let earlier = now - Duration::from_millis(50);
/// ```
///
/// Subtracting two `LinkTime` values yields a [`Duration`]:
///
/// ```no_run
/// use esp_idf_ableton_link::{Link, LinkTime};
///
/// let link = Link::new(120.0).unwrap();
/// let t1 = link.clock_now();
/// // ... some time passes ...
/// let t2 = link.clock_now();
/// let elapsed = t2 - t1; // Duration
/// ```
///
/// # FFI Access
///
/// For interoperability with the underlying C API (or other FFI), use
/// [`as_micros`](Self::as_micros):
///
/// ```no_run
/// use esp_idf_ableton_link::{Link, LinkTime};
///
/// let link = Link::new(120.0).unwrap();
/// let now = link.clock_now();
/// let micros: i64 = now.as_micros();
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkTime(i64);

impl LinkTime {
    /// Create a `LinkTime` from microseconds.
    ///
    /// This is primarily useful when interfacing with external systems that
    /// provide time values in microseconds.
    #[must_use]
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Get the time value as microseconds (signed).
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.0
    }

    /// Get the time value as an unsigned 64-bit integer (microseconds).
    ///
    /// This performs a bit-preserving cast. Link's clock is based on
    /// `steady_clock` which always returns non-negative values, so this
    /// is safe for normal use.
    #[must_use]
    const fn as_u64(self) -> u64 {
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

/// A duration of time in microseconds, for use with [`LinkTime`].
///
/// `LinkDuration` is a lightweight alternative to [`std::time::Duration`] that
/// avoids the overhead of nanosecond precision and `u128` arithmetic on
/// embedded systems.
///
/// # Creating `LinkDuration` values
///
/// ```no_run
/// use esp_idf_ableton_link::LinkDuration;
///
/// let d1 = LinkDuration::from_micros(500);
/// let d2 = LinkDuration::from_millis(10);
/// let d3 = LinkDuration::from_secs(1);
/// ```
///
/// # Arithmetic
///
/// `LinkDuration` supports multiplication and division by `i64`:
///
/// ```no_run
/// use esp_idf_ableton_link::LinkDuration;
///
/// let d = LinkDuration::from_millis(100);
/// let doubled = d * 2;
/// let halved = d / 2;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkDuration(i64);

impl LinkDuration {
    /// A duration of zero.
    pub const ZERO: Self = Self(0);

    /// Create a `LinkDuration` from microseconds.
    #[must_use]
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Create a `LinkDuration` from milliseconds.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis * 1_000)
    }

    /// Create a `LinkDuration` from seconds.
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

impl Add for LinkDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for LinkDuration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for LinkDuration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for LinkDuration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i64> for LinkDuration {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl MulAssign<i64> for LinkDuration {
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}

impl Div<i64> for LinkDuration {
    type Output = Self;

    fn div(self, rhs: i64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<i64> for LinkDuration {
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

impl Add<LinkDuration> for LinkTime {
    type Output = Self;

    fn add(self, rhs: LinkDuration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<LinkDuration> for LinkTime {
    fn add_assign(&mut self, rhs: LinkDuration) {
        *self = *self + rhs;
    }
}

impl Sub<LinkDuration> for LinkTime {
    type Output = Self;

    fn sub(self, rhs: LinkDuration) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign<LinkDuration> for LinkTime {
    fn sub_assign(&mut self, rhs: LinkDuration) {
        *self = *self - rhs;
    }
}

impl Sub<LinkTime> for LinkTime {
    type Output = LinkDuration;

    fn sub(self, rhs: LinkTime) -> Self::Output {
        LinkDuration(self.0 - rhs.0)
    }
}

impl Add<Duration> for LinkTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        let micros = i64::try_from(rhs.as_micros()).unwrap_or(i64::MAX);
        Self(self.0 + micros)
    }
}

impl AddAssign<Duration> for LinkTime {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for LinkTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        let micros = i64::try_from(rhs.as_micros()).unwrap_or(i64::MAX);
        Self(self.0 - micros)
    }
}

impl SubAssign<Duration> for LinkTime {
    fn sub_assign(&mut self, rhs: Duration) {
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

    /// Check if start/stop synchronization is enabled.
    ///
    /// When enabled, transport start/stop state is shared with other peers
    /// in the session.
    ///
    /// # Returns
    ///
    /// `true` if start/stop sync is enabled, `false` otherwise.
    #[must_use]
    pub fn is_start_stop_sync_enabled(&self) -> bool {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_is_start_stop_sync_enabled(self.handle) }
    }

    /// Enable start/stop synchronization.
    ///
    /// When enabled, transport start/stop state is shared with other peers
    /// in the session. This allows multiple applications to start and stop
    /// playback together.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// link.enable_start_stop_sync();
    /// ```
    pub fn enable_start_stop_sync(&self) {
        self.set_start_stop_sync_enabled(true);
    }

    /// Disable start/stop synchronization.
    ///
    /// See also [`enable_start_stop_sync`](Self::enable_start_stop_sync).
    pub fn disable_start_stop_sync(&self) {
        self.set_start_stop_sync_enabled(false);
    }

    /// Enable or disable start/stop synchronization.
    ///
    /// This is useful when the enabled state comes from a variable.
    /// For static enable/disable, prefer
    /// [`enable_start_stop_sync`](Self::enable_start_stop_sync) and
    /// [`disable_start_stop_sync`](Self::disable_start_stop_sync) for
    /// readability.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable start/stop sync, `false` to disable.
    pub fn set_start_stop_sync_enabled(&self, enabled: bool) {
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

    /// Register a callback to be notified when the start/stop state changes.
    ///
    /// The callback is invoked on a Link-managed thread whenever the transport
    /// start/stop state of the Link session changes.
    ///
    /// Setting a new callback replaces any previously registered callback.
    /// Use [`clear_start_stop_callback`](Self::clear_start_stop_callback) to
    /// unregister without setting a new one.
    ///
    /// # Arguments
    ///
    /// * `callback` - A closure that receives `true` when playing, `false` when stopped.
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
    /// link.enable_start_stop_sync();
    /// link.set_start_stop_callback(|is_playing| {
    ///     if is_playing {
    ///         log::info!("Transport started");
    ///     } else {
    ///         log::info!("Transport stopped");
    ///     }
    /// });
    /// link.enable();
    /// ```
    pub fn set_start_stop_callback<F>(&self, callback: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        let boxed: Box<dyn FnMut(bool) + Send> = Box::new(callback);

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

    /// Clear the start/stop callback without setting a new one.
    ///
    /// After calling this, no callback will be invoked when the start/stop state changes.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned. See
    /// [`set_start_stop_callback`](Self::set_start_stop_callback) for details.
    pub fn clear_start_stop_callback(&self) {
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
    /// The current [`LinkTime`].
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
    pub fn clock_now(&self) -> LinkTime {
        // Safety: handle is valid (checked in new()).
        LinkTime(unsafe { sys::abl_link_clock_micros(self.handle) })
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
            pub fn clock_now(&self) -> LinkTime;

            /// See [`Link::is_enabled`]. This method is realtime-safe.
            #[must_use]
            pub fn is_enabled(&self) -> bool;

            /// See [`Link::num_peers`]. This method is realtime-safe.
            #[must_use]
            pub fn num_peers(&self) -> u64;

            /// See [`Link::is_start_stop_sync_enabled`]. This method is realtime-safe.
            #[must_use]
            pub fn is_start_stop_sync_enabled(&self) -> bool;

            /// See [`Link::enable_start_stop_sync`]. This method is realtime-safe.
            pub fn enable_start_stop_sync(&self);

            /// See [`Link::disable_start_stop_sync`]. This method is realtime-safe.
            pub fn disable_start_stop_sync(&self);

            /// See [`Link::set_start_stop_sync_enabled`]. This method is realtime-safe.
            pub fn set_start_stop_sync_enabled(&self, enabled: bool);
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
/// # Usage
///
/// 1. Capture a session state with [`Link::capture_app_session_state`]
/// 2. Read values using [`tempo`](Self::tempo),
///    [`beat_at_time`](Self::beat_at_time), etc.
/// 3. Optionally modify using [`set_tempo`](Self::set_tempo),
///    [`request_beat_at_time`](Self::request_beat_at_time), etc.
/// 4. Commit changes with [`Link::commit_app_session_state`]
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
    /// # Arguments
    ///
    /// * `bpm` - The new tempo in beats per minute.
    /// * `at_time` - The time at which the tempo change takes effect.
    pub fn set_tempo(&mut self, bpm: f64, at_time: LinkTime) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_set_tempo(self.handle, bpm, at_time.as_micros()) }
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
    pub fn beat_at_time(&self, time: LinkTime, quantum: f64) -> f64 {
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
    pub fn phase_at_time(&self, time: LinkTime, quantum: f64) -> f64 {
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
    pub fn time_at_beat(&self, beat: f64, quantum: f64) -> LinkTime {
        // Safety: handle is valid (checked in new()).
        LinkTime(unsafe { sys::abl_link_time_at_beat(self.handle, beat, quantum) })
    }

    /// Request to map the given beat to the given time (quantized launch).
    ///
    /// If no other peers are connected, the beat/time mapping happens
    /// immediately. If there are other peers, this waits until the next time
    /// the session phase matches the phase of the given beat, enabling
    /// synchronized "quantized launch".
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map.
    /// * `at_time` - The requested time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn request_beat_at_time(&mut self, beat: f64, at_time: LinkTime, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_request_beat_at_time(self.handle, beat, at_time.as_micros(), quantum);
        }
    }

    /// Forcibly map the given beat to the given time for all peers.
    ///
    /// **Warning:** This is anti-social behavior that disrupts other peers.
    /// Only use this for bridging an external clock source into a Link session.
    /// Most applications should use
    /// [`request_beat_at_time`](Self::request_beat_at_time) instead.
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map.
    /// * `at_time` - The time to map it to.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn force_beat_at_time(&mut self, beat: f64, at_time: LinkTime, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_force_beat_at_time(self.handle, beat, at_time.as_u64(), quantum);
        }
    }

    /// Check if transport is playing.
    ///
    /// This is part of the start/stop sync feature.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_is_playing(self.handle) }
    }

    /// Start transport at the specified time.
    ///
    /// This is part of the start/stop sync feature.
    ///
    /// # Arguments
    ///
    /// * `at_time` - The time at which playback starts.
    pub fn play(&mut self, at_time: LinkTime) {
        self.set_playing(true, at_time);
    }

    /// Stop transport at the specified time.
    ///
    /// This is part of the start/stop sync feature.
    ///
    /// # Arguments
    ///
    /// * `at_time` - The time at which playback stops.
    pub fn stop(&mut self, at_time: LinkTime) {
        self.set_playing(false, at_time);
    }

    /// Set whether transport should be playing or stopped.
    ///
    /// This is part of the start/stop sync feature. The change takes effect at
    /// the specified time.
    ///
    /// This is useful when the playing state comes from a variable.
    /// For static play/stop, prefer [`play`](Self::play) and
    /// [`stop`](Self::stop) for readability.
    ///
    /// # Arguments
    ///
    /// * `playing` - `true` to start transport, `false` to stop.
    /// * `at_time` - The time at which the change takes effect.
    pub fn set_playing(&mut self, playing: bool, at_time: LinkTime) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_set_is_playing(self.handle, playing, at_time.as_u64()) }
    }

    /// Get the time at which the transport start/stop state last changed.
    ///
    /// # Returns
    ///
    /// The [`LinkTime`] at which the transport state change occurs.
    #[must_use]
    pub fn time_for_is_playing(&self) -> LinkTime {
        // Safety: handle is valid (checked in new()).
        LinkTime(unsafe { sys::abl_link_time_for_is_playing(self.handle) }.cast_signed())
    }

    /// Request to map the given beat to the time when transport starts playing.
    ///
    /// This is a convenience function for quantized launch scenarios. It maps
    /// the given beat to the transport start time. If transport is not playing
    /// (`is_playing` is `false`), this function is a no-op.
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map to the start time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn request_beat_at_start_playing_time(&mut self, beat: f64, quantum: f64) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_request_beat_at_start_playing_time(self.handle, beat, quantum) }
    }

    /// Set transport state and request a beat mapping in one operation.
    ///
    /// This is a convenience function that combines [`set_playing`] and
    /// [`request_beat_at_time`]. It starts or stops transport at the given
    /// time and attempts to map the given beat to that time.
    ///
    /// # Arguments
    ///
    /// * `playing` - `true` to start transport, `false` to stop.
    /// * `at_time` - The time at which the change takes effect.
    /// * `beat` - The beat to map to the given time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// [`set_playing`]: Self::set_playing
    /// [`request_beat_at_time`]: Self::request_beat_at_time
    pub fn set_playing_and_request_beat_at_time(
        &mut self,
        playing: bool,
        at_time: LinkTime,
        beat: f64,
        quantum: f64,
    ) {
        // Safety: handle is valid (checked in new()).
        unsafe {
            sys::abl_link_set_is_playing_and_request_beat_at_time(
                self.handle,
                playing,
                at_time.as_u64(),
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
