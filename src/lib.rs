//! Safe Rust wrapper for Ableton Link on ESP32 via ESP-IDF.
//!
//! This crate provides a safe Rust API for [Ableton Link](https://www.ableton.com/en/link/),
//! enabling musical applications to synchronize tempo and beat phase over a local network.
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
//! link.enable(true);
//!
//! // Get the current tempo
//! let tempo = link.tempo();
//! log::info!("Current tempo: {} BPM", tempo);
//!
//! // Get the current beat position
//! let now = Link::clock_micros();
//! let beat = link.beat_at_time(now, 4.0); // 4 beats per bar
//! log::info!("Current beat: {}", beat);
//! ```
//!
//! # ESP-IDF Component
//!
//! This crate also serves as an ESP-IDF component. The C++ wrapper and CMakeLists.txt
//! are included in the crate directory and should be referenced in your firmware's
//! `[package.metadata.esp-idf-sys].extra_components` configuration.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

use core::marker::PhantomData;

// Re-export the raw bindings for advanced users who need direct access
pub mod sys {
    pub use esp_idf_sys::ableton_link::*;
}

use esp_idf_sys::ableton_link::*;

/// Error type for Link operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Failed to allocate memory for the Link instance.
    AllocationFailed,
}

impl core::fmt::Display for LinkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "Failed to allocate Link instance"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LinkError {}

/// A safe wrapper around an Ableton Link instance.
///
/// Link enables musical applications to synchronize tempo and beat phase
/// over a local network. Multiple Link-enabled applications can play in time
/// without requiring any configuration.
///
/// # Thread Safety
///
/// `Link` is `Send` but not `Sync`. The underlying ESP32 implementation uses
/// FreeRTOS primitives that are thread-safe, but the API is designed for
/// single-threaded access to a given instance.
///
/// # Example
///
/// ```no_run
/// use esp_idf_ableton_link::Link;
///
/// let mut link = Link::new(120.0).expect("Failed to create Link");
/// link.enable(true);
///
/// // In your audio callback or main loop:
/// let now = Link::clock_micros();
/// let beat = link.beat_at_time(now, 4.0);
/// let phase = link.phase_at_time(now, 4.0);
/// ```
pub struct Link {
    handle: *mut LinkInstance,
    // PhantomData to prevent auto-impl of Send/Sync
    // We explicitly impl Send after verifying thread safety
    _marker: PhantomData<*mut ()>,
}

// Safety: Link operations go through the C++ wrapper which uses
// proper synchronization for the ESP32 platform (FreeRTOS primitives).
// The Link instance can be safely moved between threads.
unsafe impl Send for Link {}

impl Link {
    /// Create a new Link instance with the specified initial tempo.
    ///
    /// # Arguments
    ///
    /// * `initial_bpm` - The initial tempo in beats per minute (BPM).
    ///   Typical values range from 20.0 to 999.0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Link)` on success, or `Err(LinkError::AllocationFailed)` if
    /// memory allocation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).expect("Failed to create Link");
    /// ```
    pub fn new(initial_bpm: f64) -> Result<Self, LinkError> {
        // Safety: link_create is safe to call with any f64 value.
        // It returns null on allocation failure.
        let handle = unsafe { link_create(initial_bpm) };

        if handle.is_null() {
            Err(LinkError::AllocationFailed)
        } else {
            log::debug!("Created Link instance at {:p} with {} BPM", handle, initial_bpm);
            Ok(Self {
                handle,
                _marker: PhantomData,
            })
        }
    }

    /// Enable or disable Link synchronization.
    ///
    /// When enabled, Link will discover and synchronize with other Link-enabled
    /// applications on the local network.
    ///
    /// # Arguments
    ///
    /// * `enable` - `true` to enable Link, `false` to disable it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.enable(true);  // Start synchronizing
    /// // ... later ...
    /// link.enable(false); // Stop synchronizing
    /// ```
    pub fn enable(&mut self, enable: bool) {
        // Safety: handle is valid (checked in new()) and link_enable
        // handles null checks internally.
        unsafe { link_enable(self.handle, enable) }
        log::debug!("Link enabled: {}", enable);
    }

    /// Check if Link is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if Link is enabled and synchronizing, `false` otherwise.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        // Safety: handle is valid and link_is_enabled is safe to call.
        unsafe { link_is_enabled(self.handle) }
    }

    /// Get the current tempo in beats per minute (BPM).
    ///
    /// This returns the tempo from the current session state. If Link is
    /// enabled and connected to other peers, this will be the synchronized
    /// tempo.
    ///
    /// # Returns
    ///
    /// The current tempo in BPM.
    #[must_use]
    pub fn tempo(&self) -> f64 {
        // Safety: handle is valid and link_get_tempo is safe to call.
        unsafe { link_get_tempo(self.handle) }
    }

    /// Set the tempo in beats per minute (BPM).
    ///
    /// When Link is enabled, this change will be propagated to all connected
    /// peers on the network.
    ///
    /// # Arguments
    ///
    /// * `bpm` - The new tempo in BPM. Typical values range from 20.0 to 999.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let mut link = Link::new(120.0).unwrap();
    /// link.set_tempo(140.0); // Change tempo to 140 BPM
    /// ```
    pub fn set_tempo(&mut self, bpm: f64) {
        // Safety: handle is valid and link_set_tempo is safe to call.
        unsafe { link_set_tempo(self.handle, bpm) }
        log::debug!("Set tempo to {} BPM", bpm);
    }

    /// Get the beat value at a given time.
    ///
    /// The beat value is a continuous floating-point number that increases
    /// with time according to the current tempo. It can be used to determine
    /// the current position in the musical timeline.
    ///
    /// # Arguments
    ///
    /// * `micros` - The time in microseconds (from [`Link::clock_micros()`]).
    /// * `quantum` - The quantum (number of beats per cycle/bar). Common values
    ///   are 4.0 (4/4 time) or 3.0 (3/4 time).
    ///
    /// # Returns
    ///
    /// The beat value at the specified time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// let now = Link::clock_micros();
    /// let beat = link.beat_at_time(now, 4.0);
    /// log::info!("Current beat: {}", beat);
    /// ```
    #[must_use]
    pub fn beat_at_time(&self, micros: i64, quantum: f64) -> f64 {
        // Safety: handle is valid and link_get_beat_at_time is safe to call.
        unsafe { link_get_beat_at_time(self.handle, micros, quantum) }
    }

    /// Get the phase (position within a cycle) at a given time.
    ///
    /// The phase is a value from 0.0 to the quantum that represents the current
    /// position within a bar or cycle. This is useful for synchronizing visual
    /// effects or triggering events at specific points in the bar.
    ///
    /// # Arguments
    ///
    /// * `micros` - The time in microseconds (from [`Link::clock_micros()`]).
    /// * `quantum` - The quantum (number of beats per cycle/bar).
    ///
    /// # Returns
    ///
    /// The phase value (0.0 to quantum) at the specified time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use esp_idf_ableton_link::Link;
    ///
    /// let link = Link::new(120.0).unwrap();
    /// let now = Link::clock_micros();
    /// let phase = link.phase_at_time(now, 4.0);
    ///
    /// // Check if we're at the start of a bar (with some tolerance)
    /// if phase < 0.1 || phase > 3.9 {
    ///     log::info!("At the downbeat!");
    /// }
    /// ```
    #[must_use]
    pub fn phase_at_time(&self, micros: i64, quantum: f64) -> f64 {
        // Safety: handle is valid and link_get_phase_at_time is safe to call.
        unsafe { link_get_phase_at_time(self.handle, micros, quantum) }
    }

    /// Get the current Link clock time in microseconds.
    ///
    /// This returns the current time from Link's internal clock, which is
    /// synchronized across all connected peers. Use this value as input to
    /// [`beat_at_time()`](Self::beat_at_time) and [`phase_at_time()`](Self::phase_at_time).
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
    /// let now = Link::clock_micros();
    /// log::info!("Current Link clock: {} µs", now);
    /// ```
    #[must_use]
    pub fn clock_micros() -> i64 {
        // Safety: link_clock_micros has no preconditions and is always safe to call.
        unsafe { link_clock_micros() }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        log::debug!("Destroying Link instance at {:p}", self.handle);
        // Safety: handle is valid (checked in new()) and link_destroy
        // handles null checks internally. After this call, handle is invalid
        // but that's fine since we're being dropped.
        unsafe { link_destroy(self.handle) }
    }
}
