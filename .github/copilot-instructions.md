# Copilot Instructions

## API Review TODO

Work through these items one by one:

1. [x] **`i64`/`u64` time inconsistency** — Introduced `LinkTime` newtype wrapping `i64` microseconds. Renamed `clock_micros()` to `clock_now()`. All time parameters/returns now use `LinkTime`. Supports `Duration` arithmetic.

2. [x] **`set_is_playing` naming** — Added `play()`/`stop()` convenience methods and renamed `set_is_playing()` to `set_playing()`. Also renamed `set_is_playing_and_request_beat_at_time()` to `set_playing_and_request_beat_at_time()`.

3. [x] **`SessionState` is `Send` but not `Sync`** — Verified intentional: C API doesn't document thread-safety for concurrent reads, and session state is designed for local/scoped use. Added comment explaining why `Sync` is not implemented.

4. [ ] **Callback API** — Consider returning a guard that clears the callback on drop (RAII pattern) instead of separate `set_*_callback`/`clear_*_callback` methods.

5. [ ] **`commit_app_session_state` takes `&mut self`** — May be unnecessary given the underlying C++ is thread-safe. Reconsider if `&self` would be more appropriate.

6. [x] **Missing `play()`/`stop()` convenience methods on `SessionState`** — Similar to the `enable()`/`disable()`/`set_enabled()` pattern we just added.

7. [ ] **`set_playing_and_request_beat_at_time` and similar combined methods** — Review whether these convenience methods pulling in `Link` are the right API design, or if users should compose the operations themselves.
