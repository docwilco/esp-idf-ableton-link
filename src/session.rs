//! Session state for Link synchronization.

use crate::time::Instant;
use crate::TransportState;

mod sys {
    #[allow(clippy::wildcard_imports)]
    pub use esp_idf_sys::abl_link::*;
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
/// 1. Capture a session state with [`Link::capture_app_session_state`](crate::Link::capture_app_session_state)
///    or [`AudioLink::capture_session_state`](crate::AudioLink::capture_session_state)
/// 2. Read values using [`tempo`](Self::tempo),
///    [`beat_at_time`](Self::beat_at_time), [`transport_state`](Self::transport_state), etc.
/// 3. Optionally modify using [`set_tempo`](Self::set_tempo),
///    [`request_beat_at_time`](Self::request_beat_at_time),
///    [`set_transport_state_at`](Self::set_transport_state_at), etc.
/// 4. Commit changes with [`Link::commit_app_session_state`](crate::Link::commit_app_session_state)
///    or [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state)
///
/// # Important
///
/// This is a snapshot and will become stale. Don't store it for later use.
/// Capture a fresh state when you need current values.
pub struct SessionState {
    pub(crate) handle: sys::abl_link_session_state,
}

// Safety: SessionState is an independent snapshot with no references to Link.
// It can be safely moved between threads.
//
// Note: Sync is intentionally NOT implemented. The underlying C API does not
// document thread-safety for concurrent reads of session state, and the design
// intent is for session state to be used in a local scope after capture.
unsafe impl Send for SessionState {}

impl SessionState {
    /// Create a new `SessionState` from a raw handle.
    ///
    /// # Safety
    ///
    /// The handle must be valid (non-null `impl_` pointer).
    pub(crate) const fn from_handle(handle: sys::abl_link_session_state) -> Self {
        Self { handle }
    }

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
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// * `time` - The time (from [`Link::clock_now`](crate::Link::clock_now)).
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
    /// * `time` - The time (from [`Link::clock_now`](crate::Link::clock_now)).
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
        Instant::from_micros(unsafe { sys::abl_link_time_at_beat(self.handle, beat, quantum) })
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
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// [`Link::enable_transport_sync`](crate::Link::enable_transport_sync) to share state with peers.
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
    /// [`Link::enable_transport_sync`](crate::Link::enable_transport_sync) to share state with peers.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// [`Link::enable_transport_sync`](crate::Link::enable_transport_sync) to share state with peers.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// [`Link::enable_transport_sync`](crate::Link::enable_transport_sync) to share state with peers. The change
    /// takes effect at the specified time.
    ///
    /// This is useful when the transport state comes from a variable.
    /// For static start/stop, prefer [`start_transport_at`](Self::start_transport_at)
    /// and [`stop_transport_at`](Self::stop_transport_at) for readability.
    ///
    /// Changes are local to this snapshot until committed with
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
        Instant::from_micros(
            unsafe { sys::abl_link_time_for_is_playing(self.handle) }.cast_signed(),
        )
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
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
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
    /// [`Link::commit_app_session_state`](crate::Link::commit_app_session_state) or
    /// [`AudioLink::commit_session_state`](crate::AudioLink::commit_session_state).
    ///
    /// # Arguments
    ///
    /// * `beat` - The beat to map to the start time.
    /// * `time` - The time at which transport starts.
    /// * `quantum` - The quantum (beats per cycle/bar).
    ///
    /// [`start_transport_at`]: Self::start_transport_at
    /// [`request_beat_at_transport_state_time`]: Self::request_beat_at_transport_state_time
    pub fn start_transport_and_request_beat_at(&mut self, beat: f64, time: Instant, quantum: f64) {
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
