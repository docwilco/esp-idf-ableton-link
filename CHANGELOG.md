# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0-beta.2] - 2026-04-17

### Changed

- **Breaking:** Updated `esp-idf-sys` dependency from 0.36 to 0.37.2.

## [0.2.1-beta2] - 2026-03-06

### Changed

- Wraps Ableton Link 4.0.0-beta.2 (was 4.0.0-beta.1).
  - Discovery now works across subnets.
  - ESP-IDF 6.0 compatibility.
- Now correctly lists supported targets. Added supports RISC-V targets: ESP32-C5, ESP32-C6, ESP32-C61, ESP32-H2, ESP32-P4. Removed ESP32-S2.

## [0.2.0-beta1] - 2025-03-03

### Changed

- **Breaking:** `AudioLink` renamed to `RealtimeHandle`. This better
  describes the type's purpose (a handle for realtime-safe access) and
  avoids confusion with upstream Link 4.0's new `LinkAudio` API. `LinkAudio` does not have a C API yet, so this crate does not wrap it at this time.
- **Breaking:** `Link::bind_audio_thread()` renamed to
  `Link::bind_realtime()`. Matches the `RealtimeHandle` rename.
- Wraps Ableton Link 4.0.0-beta.1 (was 3.1.5).
- All upstream time parameters now consistently use `i64` microseconds
  (was a mix of `i64` and `u64`). No public API impact — the `Instant`
  newtype already abstracted over the underlying integer type.

## [0.1.0] - 2025-02-01

Initial release wrapping Ableton Link 3.1.5 via ESP-IDF component
`docwilco/esp_abl_link` 3.1.5.

- Safe Rust API for Ableton Link on ESP32 (Xtensa targets)
- `Link` type with thread-safe tempo/beat synchronization
- `SessionState` for capturing and committing session snapshots
- `AudioLink` handle for realtime audio-thread access
- `Instant` and `Duration` newtypes optimized for 32-bit embedded targets
- Transport state synchronization with `TransportState` enum
- Callbacks for peer count, tempo, and transport state changes

[0.3.0-beta.2]: https://github.com/docwilco/esp-idf-ableton-link/compare/v0.2.1-beta2...v0.3.0-beta.2
[0.2.1-beta2]: https://github.com/docwilco/esp-idf-ableton-link/compare/v0.2.0-beta.1...v0.2.1-beta2
[0.2.0-beta1]: https://github.com/docwilco/esp-idf-ableton-link/compare/v0.1.0...v0.2.0-beta1
[0.1.0]: https://github.com/docwilco/esp-idf-ableton-link/releases/tag/v0.1.0
