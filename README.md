# esp-idf-ableton-link

[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://docwilco.github.io/esp-idf-ableton-link/)

Safe Rust wrapper for Ableton Link on ESP32 via ESP-IDF.

This crate provides a safe Rust API for [Ableton
Link](https://www.ableton.com/en/link/), enabling musical applications to
synchronize tempo and beat phase over a local network on ESP32 hardware.

## Supported Hardware

Currently only Xtensa-based ESP32 chips are supported:

- ESP32
- ESP32-S2
- ESP32-S3

## Usage

It is recommended to start your project using the
[esp-idf-template](https://github.com/esp-rs/esp-idf-template?tab=readme-ov-file#generate-the-project).

### Required Configuration

This crate requires specific configuration in your ESP32 project. All four
steps below are mandatory.

#### 1. Add the crate and build dependency

```sh
cargo add esp-idf-ableton-link
cargo add --build embuild
```

#### 2. Set `ESP_IDF_SYS_ROOT_CRATE` in `.cargo/config.toml`

The `ESP_IDF_SYS_ROOT_CRATE` environment variable must be set so that
`esp-idf-sys` can discover the extra component configuration from this
crate.

Add to your project's `.cargo/config.toml`:

```toml
[build]
target = "xtensa-esp32-espidf"

[target.xtensa-esp32-espidf]
linker = "ldproxy"
runner = "espflash flash --monitor"
rustflags = [ "--cfg",  "espidf_time64"]

[env]
MCU="esp32"
ESP_IDF_VERSION = "v5.3.3"
ESP_IDF_SYS_ROOT_CRATE = "your-firmware-crate-name"
```

Replace `your-firmware-crate-name` with the `name` field from your
project's `Cargo.toml`.

For more information, see the [ESP-IDF configuration
documentation](https://github.com/esp-rs/esp-idf-sys/blob/master/BUILD-OPTIONS.md#esp-idf-configuration).

#### 3. Enable C++ Exceptions in `sdkconfig.defaults`

Ableton Link requires C++ exception support. Add this to your project's
`sdkconfig.defaults`:

```
CONFIG_COMPILER_CXX_EXCEPTIONS=y
```

You may also want to increase the main task stack size (Rust often needs
more than the default 3KB):

```
CONFIG_ESP_MAIN_TASK_STACK_SIZE=8000
```

For more information about ESP-IDF sdkconfig options, see the [ESP-IDF
KConfig
reference](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/kconfig.html).

#### 4. Add `embuild::espidf::sysenv::output()` to your `build.rs`

Your project's `build.rs` must call `embuild::espidf::sysenv::output()` to
propagate ESP-IDF configuration flags (such as
`esp_idf_compiler_cxx_exceptions`) to the Rust compiler:

```rust
fn main() {
    embuild::espidf::sysenv::output();
}
```

## Example

```rust
use esp_idf_ableton_link::Link;

// Create a new Link instance with 120 BPM
let mut link = Link::new(120.0).expect("Failed to create Link");

// Enable Link to start synchronizing
link.enable();

// Wait for Link to discover and sync with any existing session
esp_idf_svc::hal::delay::FreeRtos::delay_ms(4000);

// Capture session state and read tempo
let state = link.capture_app_session_state().unwrap();
let tempo = state.tempo();
log::info!("Current tempo: {} BPM", tempo);

// Get the current beat position
let now = link.clock_now();
let beat = state.beat_at_time(now, 4.0); // 4 beats per bar
log::info!("Current beat: {}", beat);
```

## Application vs Audio Thread Session State

Link provides two sets of session state functions for different contexts:

- **Application thread functions** ([`Link::capture_app_session_state`],
  [`Link::commit_app_session_state`]): For use from general application
  code. These may briefly block to synchronize with the audio thread.
  They are thread-safe and can be called from multiple threads/tasks.

- **Audio thread functions** ([`AudioLink::capture_session_state`],
  [`AudioLink::commit_session_state`]): For use from realtime audio
  callbacks. These are lock-free and will never block, making them safe
  for low-latency audio processing.

If your application has a dedicated audio thread with realtime constraints,
use [`Link::bind_audio_thread`] to obtain an [`AudioLink`] handle and use
its methods exclusively from that thread. For simpler applications without
strict realtime requirements, the application thread functions are
sufficient.

The Link library recommends avoiding concurrent session state modifications
from both application and audio threads. This crate enforces that
recommendation: the [`AudioLink`] handle mutably borrows the [`Link`]
instance, preventing concurrent access at compile time.

## The Timeline

Link maintains a **timeline**—a mapping from wall-clock time to beat values.
This timeline is synchronized across all peers in a session, enabling
applications to know "where we are in the music" at any given moment.

### Tempo

The timeline progresses at a rate determined by the **tempo** (beats per
minute). At 120 BPM, beat values increase by 2.0 per second. Tempo changes
are propagated to all peers.

### Beats

Beat values are continuous `f64` numbers. The integer part is the number of
completed beats; the fractional part is the position within the current beat.
For example, `4.75` means 4 beats completed plus three-quarters through the
5th beat.

Each peer has its own beat origin, so absolute beat values differ between
peers. What's synchronized is the **phase** (see below).

### Quantum and Phase

- **Quantum**: The number of beats per musical cycle (typically a bar).
  Common values:
  - `4.0` — 4/4 time (most electronic music, rock, pop)
  - `3.0` — 3/4 time (waltz, some ballads)
  - `6.0` — 6/8 time (jigs, some ballads)

- **Phase**: Position within the current cycle, in range `[0, quantum)`.
  Conceptually `beat % quantum` (but handles negatives correctly).
  For example, with quantum `4.0`:
  - Beat `0.0` → phase `0.0` (downbeat)
  - Beat `1.5` → phase `1.5` (halfway through beat 2)
  - Beat `4.0` → phase `0.0` (next downbeat)
  - Beat `7.25` → phase `3.25` (one quarter through beat 4)
  - Beat `-1.0` → phase `3.0` (one beat before the downbeat)

Quantum is a **local parameter**—each peer chooses its own. Peers using the
same quantum will have synchronized phases (downbeats align). Peers using
different quantums only align at beats divisible by both.

For example, if Peer A uses quantum `4.0` and Peer B uses quantum `3.0`:
- At beat 12.0: both are at phase 0.0 (downbeat) ✓
- At beat 15.0: A is at phase 3.0, B is at phase 0.0 (downbeat) ✗
- At beat 16.0: A is at phase 0.0 (downbeat), B is at phase 1.0 ✗
- At beat 24.0: both are at phase 0.0 (downbeat) ✓

Their downbeats only align at multiples of 12 (the LCM of 3 and 4).

### Beat Magnitude vs Phase

Each Link peer maintains its own beat timeline with an arbitrary origin.
The absolute beat value (magnitude) will differ between peers, but the
**phase** (position within the cycle) is synchronized across all peers.

For example, with quantum `4.0`:
- Peer A might be at beat `12.5` (phase `0.5`)
- Peer B might be at beat `100.5` (phase `0.5`)
- Peer C might be at beat `0.5` (phase `0.5`)

All three are synchronized: they're all halfway through the first beat of
their respective bars.

### Shifting the Timeline

When you "request a beat at a time" via [`SessionState::request_beat_at_time`],
you're shifting the entire timeline so that beat aligns with that time. This
affects beat values at all times (past, present, future). If you map beat
`0.0` to a time 2 beats in the future, the current beat becomes `-2.0`.

### Practical Implications

- Use [`SessionState::phase_at_time`] when you need to know where you are in
  a bar/cycle (e.g., for triggering events on the downbeat).
- Use [`SessionState::beat_at_time`] when you need to track continuous
  progress or count beats locally.
- Use [`SessionState::request_beat_at_time`] to adjust your local beat value
  while preserving phase alignment with the session.

### Examples

#### Triggering on the Downbeat

To find the exact time of the next downbeat and schedule an action:

```rust
let quantum = 4.0; // 4/4 time
let state = link.capture_app_session_state().unwrap();

// Find the next downbeat after the current time
let now = link.clock_now();
let current_beat = state.beat_at_time(now, quantum);
let current_phase = state.phase_at_time(now, quantum);
let next_downbeat_beat = current_beat + (quantum - current_phase);
let next_downbeat_time = state.time_at_beat(next_downbeat_beat, quantum);

// Compute delay for use with ESP-IDF timer APIs
let delay_us = (next_downbeat_time - now).as_micros();
// esp_timer_start_once(timer_handle, delay_us as u64);
```

#### Counting Beats Locally

To count how many beats have elapsed since your app started:

```rust
let start_time = link.clock_now();
let state = link.capture_app_session_state().unwrap();
let start_beat = state.beat_at_time(start_time, quantum);

// Later...
let now = link.clock_now();
let state = link.capture_app_session_state().unwrap();
let current_beat = state.beat_at_time(now, quantum);
let beats_elapsed = current_beat - start_beat;
```

#### Aligning to a Specific Beat

To make your local beat counter start at 0 on the next downbeat:

```rust
let quantum = 4.0;
let now = link.clock_now();
let mut state = link.capture_app_session_state().unwrap();

// Request that beat 0.0 occurs at the current time.
// - If alone: beat 0.0 is mapped to `now` immediately.
// - If with peers: beat 0.0 is mapped to the next time the session
//   phase is 0.0 (the next downbeat), preserving sync.
state.request_beat_at_time(0.0, now, quantum);
link.commit_app_session_state(&state);
```

## Transport State

Separately from the timeline, Link can synchronize **transport state**
(play/stop). This is opt-in via [`Link::enable_transport_sync`].

### The Transport State Model

Transport state consists of two pieces:
- A [`TransportState`] ([`Play`](TransportState::Play) or [`Stop`](TransportState::Stop))
- A timestamp indicating when that state took/takes effect

The state may be **currently active** (timestamp in the past) or **scheduled**
(timestamp in the future). Use [`SessionState::transport_state`] to get the
state and [`SessionState::transport_state_time`] to determine when:

```rust
let state = session.transport_state();      // Play or Stop
let when = session.transport_state_time();  // When it took/takes effect
let now = link.clock_now();

if when < now {
    // State is currently active
} else {
    // State is scheduled for the future
}
```

### Transport State is Independent of the Timeline

Transport state does **not** affect the timeline. The beat/time mapping
continues unchanged regardless of whether transport is playing or stopped.
It's up to your application to decide what "playing" and "stopped" mean
(e.g., producing sound or not).

### When Transport Sync is Disabled

Even with transport sync disabled ([`Link::disable_transport_sync`]), you
can still use the transport state API locally:

- [`SessionState::set_transport_state_at`] and related methods work normally
- [`SessionState::transport_state`] and [`SessionState::transport_state_time`]
  reflect your local changes after committing

The difference is that your transport state changes won't be broadcast to
peers, and you won't receive transport state changes from peers. This can
be useful for tracking play/stop state locally without participating in
session-wide transport sync.

## Naming Differences from the C/C++ API

This crate uses Rust-idiomatic naming that differs from the original C API
(`abl_link.h`) and C++ API (`Link.hpp`). Key differences:

| This crate | C API | C++ API | Notes |
|------------|-------|---------|-------|
| [`TransportState`] | `bool` | `bool` | We use an enum with [`Play`](TransportState::Play)/[`Stop`](TransportState::Stop) variants |
| [`transport_state`](SessionState::transport_state) | `abl_link_is_playing` | `isPlaying` | Returns [`TransportState`] enum |
| [`set_transport_state_at`](SessionState::set_transport_state_at) | `abl_link_set_is_playing` | `setIsPlaying` | Takes [`TransportState`] enum |
| [`transport_state_time`](SessionState::transport_state_time) | `abl_link_time_for_is_playing` | `timeForIsPlaying` | Original name is confusing |
| [`enable_transport_sync`](Link::enable_transport_sync) | `abl_link_enable_start_stop_sync` | `enableStartStopSync` | "Transport" is more descriptive |
| [`is_transport_sync_enabled`](Link::is_transport_sync_enabled) | `abl_link_is_start_stop_sync_enabled` | `isStartStopSyncEnabled` | |
| [`set_transport_state_callback`](Link::set_transport_state_callback) | `abl_link_set_start_stop_callback` | `setStartStopCallback` | |
| [`Instant`] | `int64_t` / `uint64_t` | `std::chrono::microseconds` | Newtype for type safety and clarity |
| [`clock_now`](Link::clock_now) | `abl_link_clock_micros` | `clock().micros()` | Returns [`Instant`] |

The C API uses "is playing" terminology because transport state is
represented as a boolean. We chose [`TransportState`] with explicit
[`Play`](TransportState::Play)/[`Stop`](TransportState::Stop) variants for
clarity, since the state can be either currently active or scheduled for the
future (see [The Transport State Model](#the-transport-state-model)).

## License

GPL-2.0-or-later. See [LICENSE.md](LICENSE.md) for details.
