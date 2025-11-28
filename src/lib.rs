//! Safe Rust wrapper for Ableton Link on ESP32 via ESP-IDF.
//!
//! This crate provides a safe Rust API for [Ableton
//! Link](https://www.ableton.com/en/link/), enabling musical applications to
//! synchronize tempo and beat phase over a local network.
//!
//! # Example
//!
//! ```no_run
//! use esp_idf_ableton_link::Link;
//!
//! // Create a new Link instance with 120 BPM
//! let mut link = Link::new(120.0).expect("Failed to create Link");
//!
//! // Enable Link to start synchronizing
//! link.enable();
//!
//! // Capture session state and read tempo
//! let state = link.capture_app_session_state().unwrap();
//! let tempo = state.tempo();
//! log::info!("Current tempo: {} BPM", tempo);
//!
//! // Get the current beat position
//! let now = link.clock_micros();
//! let beat = state.beat_at_time(now, 4.0); // 4 beats per bar
//! log::info!("Current beat: {}", beat);
//! ```
//!
//! # ESP-IDF Component
//!
//! This crate also serves as an ESP-IDF component. The official `abl_link` C
//! wrapper from Ableton Link is used, and CMakeLists.txt is included in the
//! crate directory and should be referenced in your firmware's
//! `[package.metadata.esp-idf-sys].extra_components` configuration.
//!
//! # Required Configuration
//!
//! ## C++ Exceptions
//!
//! Ableton Link requires C++ exception support. Add this to your
//! `sdkconfig.defaults`:
//!
//! ```text
//! CONFIG_COMPILER_CXX_EXCEPTIONS=y
//! ```
//!
//! ## Root Crate Configuration
//!
//! If you're using a virtual workspace (no root package), you must set
//! `ESP_IDF_SYS_ROOT_CRATE` in your `.cargo/config.toml`:
//!
//! ```toml
//! [env]
//! ESP_IDF_SYS_ROOT_CRATE = "your-firmware-crate-name"
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(esp_idf_compiler_cxx_exceptions))]
compile_error!(
    r#"
================================================================================
                    ESP-IDF ABLETON LINK CONFIGURATION ERROR
================================================================================

Ableton Link requires C++ exception support, but CONFIG_COMPILER_CXX_EXCEPTIONS
is not enabled in your ESP-IDF configuration.

To fix this, add the following line to your project's `sdkconfig.defaults` file:

    CONFIG_COMPILER_CXX_EXCEPTIONS=y

Then clean and rebuild:

    cargo clean
    cargo build

WHY THIS IS REQUIRED:
---------------------
Ableton Link's C++ implementation uses exceptions for error handling. Without
exception support enabled in ESP-IDF, the Link library will fail to compile.

LOCATION:
---------
Create or edit `sdkconfig.defaults` in your project root (the same directory
as your main Cargo.toml).

For more information about ESP-IDF sdkconfig options, see:
https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/kconfig.html

================================================================================
"#
);

#[cfg(feature = "std")]
extern crate std;

use core::marker::PhantomData;

// Re-export the raw bindings for advanced users who need direct access
pub mod sys {
    // Allow wildcard imports for the sys module since there is nothing else in
    // this module.
    #[allow(clippy::wildcard_imports)]
    pub use esp_idf_sys::ableton_link::*;
}

/// Error type for Link operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Failed to allocate memory for the Link instance.
    AllocationFailed,
    /// Failed to capture session state.
    SessionStateCaptureError,
}

impl core::fmt::Display for LinkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "Failed to allocate Link instance"),
            Self::SessionStateCaptureError => write!(f, "Failed to capture session state"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinkError {}

/// A safe wrapper around an Ableton Link instance.
///
/// Link enables musical applications to synchronize tempo and beat phase over a
/// local network. Multiple Link-enabled applications can play in time without
/// requiring any configuration.
///
/// # Thread Safety
///
/// `Link` is `Send` but not `Sync`. The underlying ESP32 implementation uses
/// `FreeRTOS` primitives that are thread-safe, but the API is designed for
/// single-threaded access to a given instance.
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
/// let now = link.clock_micros();
/// let beat = state.beat_at_time(now, 4.0);
/// let phase = state.phase_at_time(now, 4.0);
///
/// // Modify and commit changes
/// state.set_tempo(140.0, now);
/// link.commit_app_session_state(&state);
/// ```
pub struct Link {
    handle: sys::abl_link,
    // PhantomData to prevent auto-impl of Send/Sync We explicitly impl Send
    // after verifying thread safety
    _marker: PhantomData<*mut ()>,
}

// Safety: We hold an abl_link struct containing a pointer to a heap-allocated
// C++ Link object. All Link methods we use (enable, isEnabled,
// captureAppSessionState, commitAppSessionState) are documented as
// "Thread-safe: yes" in abl_link.h. Moving the Rust wrapper between threads is
// safe because the underlying C++ operations use proper synchronization
// internally.
unsafe impl Send for Link {}

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
                _marker: PhantomData,
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
    /// let mut link = Link::new(120.0).unwrap();
    /// link.enable();  // Start synchronizing
    /// // ... later ...
    /// link.disable(); // Stop synchronizing
    /// ```
    pub fn enable(&mut self) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_enable(self.handle, true) }
        log::debug!("Link enabled");
    }

    /// Disable Link synchronization.
    ///
    /// See also [`enable`](Self::enable).
    pub fn disable(&mut self) {
        // Safety: handle is valid (checked in new()).
        unsafe { sys::abl_link_enable(self.handle, false) }
        log::debug!("Link disabled");
    }

    /// Check if Link is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if Link is enabled and synchronizing, `false` otherwise.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        // Safety: handle is valid.
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
        // Safety: handle is valid.
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
        // Safety: handle is valid.
        unsafe { sys::abl_link_is_start_stop_sync_enabled(self.handle) }
    }

    /// Enable or disable start/stop synchronization.
    ///
    /// When enabled, transport start/stop state is shared with other peers
    /// in the session. This allows multiple applications to start and stop
    /// playback together.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable start/stop sync, `false` to disable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.enable_start_stop_sync(true);
    /// ```
    pub fn enable_start_stop_sync(&mut self, enabled: bool) {
        // Safety: handle is valid.
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
    /// Returns [`LinkError::SessionStateCaptureError`] if the session state
    /// could not be captured (typically due to memory exhaustion).
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
            return Err(LinkError::SessionStateCaptureError);
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
    /// let now = link.clock_micros();
    /// state.set_tempo(140.0, now);
    /// link.commit_app_session_state(&state);
    /// ```
    pub fn commit_app_session_state(&mut self, state: &SessionState) {
        // Safety: both handles are valid.
        unsafe { sys::abl_link_commit_app_session_state(self.handle, state.handle) }
    }

    /// Get the current Link clock time in microseconds.
    ///
    /// This returns the current time from Link's internal clock, which is
    /// synchronized across all connected peers. Use this value as input to
    /// [`SessionState::beat_at_time`] and [`SessionState::phase_at_time`].
    ///
    /// # Returns
    ///
    /// The current time in microseconds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// let now = link.clock_micros();
    /// log::info!("Current Link clock: {} µs", now);
    /// ```
    #[must_use]
    pub fn clock_micros(&self) -> i64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_clock_micros(self.handle) }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        log::debug!("Destroying Link instance at {:p}", self.handle.impl_);
        // Safety: handle is valid (checked in new()). After this call, handle
        // is invalid but that's fine since we're being dropped.
        unsafe { sys::abl_link_destroy(self.handle) }
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
unsafe impl Send for SessionState {}

impl SessionState {
    /// Get the tempo of the timeline in Beats Per Minute.
    ///
    /// This is a stable value appropriate for display to the user. Beat time
    /// progress may not match this tempo exactly due to clock drift
    /// compensation.
    #[must_use]
    pub fn tempo(&self) -> f64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_tempo(self.handle) }
    }

    /// Set the timeline tempo to the given BPM value.
    ///
    /// # Arguments
    ///
    /// * `bpm` - The new tempo in beats per minute.
    /// * `at_time_micros` - The time at which the tempo change takes effect.
    pub fn set_tempo(&mut self, bpm: f64, at_time_micros: i64) {
        // Safety: handle is valid.
        unsafe { sys::abl_link_set_tempo(self.handle, bpm, at_time_micros) }
    }

    /// Get the beat value at the given time for the given quantum.
    ///
    /// The beat value's magnitude is unique to this Link instance, but its
    /// phase with respect to the quantum is shared among all session peers.
    ///
    /// # Arguments
    ///
    /// * `micros` - The time in microseconds (from [`Link::clock_micros`]).
    /// * `quantum` - The quantum (beats per cycle/bar).
    #[must_use]
    pub fn beat_at_time(&self, micros: i64, quantum: f64) -> f64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_beat_at_time(self.handle, micros, quantum) }
    }

    /// Get the phase (position within a cycle) at the given time.
    ///
    /// The result is in the interval `[0, quantum)`. This is equivalent to
    /// `beat_at_time(t, q) % q` for non-negative beat values, but handles
    /// negative values correctly.
    ///
    /// # Arguments
    ///
    /// * `micros` - The time in microseconds (from [`Link::clock_micros`]).
    /// * `quantum` - The quantum (beats per cycle/bar).
    #[must_use]
    pub fn phase_at_time(&self, micros: i64, quantum: f64) -> f64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_phase_at_time(self.handle, micros, quantum) }
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
    pub fn time_at_beat(&self, beat: f64, quantum: f64) -> i64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_time_at_beat(self.handle, beat, quantum) }
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
    /// * `at_time_micros` - The requested time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    pub fn request_beat_at_time(&mut self, beat: f64, at_time_micros: i64, quantum: f64) {
        // Safety: handle is valid.
        unsafe {
            sys::abl_link_request_beat_at_time(self.handle, beat, at_time_micros, quantum);
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
    /// * `at_time_micros` - The time to map it to.
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// Note: `at_time_micros` is `uint64_t` in the C API, so we match that here
    /// with `u64`. The C++ uses `std::chrono::microseconds` which is signed,
    /// but if the lack of negative times is an issue, please open an issue.
    pub fn force_beat_at_time(&mut self, beat: f64, at_time_micros: u64, quantum: f64) {
        // Safety: handle is valid.
        //
        unsafe {
            sys::abl_link_force_beat_at_time(self.handle, beat, at_time_micros, quantum);
        }
    }

    /// Check if transport is playing.
    ///
    /// This is part of the start/stop sync feature.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        // Safety: handle is valid.
        unsafe { sys::abl_link_is_playing(self.handle) }
    }

    /// Set whether transport should be playing or stopped.
    ///
    /// This is part of the start/stop sync feature. The change takes effect at
    /// the specified time.
    ///
    /// # Arguments
    ///
    /// * `is_playing` - `true` to start transport, `false` to stop.
    /// * `at_time_micros` - The time at which the change takes effect.
    ///
    /// Note: `at_time_micros` is `uint64_t` in the C API, so we match that here
    /// with `u64`. The C++ uses `std::chrono::microseconds` which is signed,
    /// but if the lack of negative times is an issue, please open an issue.
    pub fn set_is_playing(&mut self, is_playing: bool, at_time_micros: u64) {
        // Safety: handle is valid.
        unsafe { sys::abl_link_set_is_playing(self.handle, is_playing, at_time_micros) }
    }

    /// Get the time at which the transport start/stop state last changed.
    ///
    /// # Returns
    ///
    /// The time in microseconds at which the transport state change occurs.
    ///
    /// Note: Returns `u64` to match the C API (`uint64_t`).
    #[must_use]
    pub fn time_for_is_playing(&self) -> u64 {
        // Safety: handle is valid.
        unsafe { sys::abl_link_time_for_is_playing(self.handle) }
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
        // Safety: handle is valid.
        unsafe { sys::abl_link_request_beat_at_start_playing_time(self.handle, beat, quantum) }
    }

    /// Set transport state and request a beat mapping in one operation.
    ///
    /// This is a convenience function that combines [`set_is_playing`] and
    /// [`request_beat_at_time`]. It starts or stops transport at the given
    /// time and attempts to map the given beat to that time.
    ///
    /// # Arguments
    ///
    /// * `is_playing` - `true` to start transport, `false` to stop.
    /// * `at_time_micros` - The time at which the change takes effect.
    /// * `beat` - The beat to map to the given time.
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// Note: `at_time_micros` is `uint64_t` in the C API, so we match that here
    /// with `u64`. The C++ uses `std::chrono::microseconds` which is signed,
    /// but if the lack of negative times is an issue, please open an issue.
    ///
    /// [`set_is_playing`]: Self::set_is_playing
    /// [`request_beat_at_time`]: Self::request_beat_at_time
    pub fn set_is_playing_and_request_beat_at_time(
        &mut self,
        is_playing: bool,
        at_time_micros: u64,
        beat: f64,
        quantum: f64,
    ) {
        // Safety: handle is valid.
        unsafe {
            sys::abl_link_set_is_playing_and_request_beat_at_time(
                self.handle,
                is_playing,
                at_time_micros,
                beat,
                quantum,
            );
        }
    }
}

impl Drop for SessionState {
    fn drop(&mut self) {
        // Safety: handle is valid.
        unsafe { sys::abl_link_destroy_session_state(self.handle) }
    }
}
