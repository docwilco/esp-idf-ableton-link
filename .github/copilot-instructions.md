# Copilot Instructions

## API Review TODO

Work through these items one by one:

1. [x] **`i64`/`u64` time inconsistency** — Introduced `LinkTime` newtype wrapping `i64` microseconds. Renamed `clock_micros()` to `clock_now()`. All time parameters/returns now use `LinkTime`. Supports `Duration` arithmetic.

2. [x] **`set_is_playing` naming** — Added `play()`/`stop()` convenience methods and renamed `set_is_playing()` to `set_playing()`. Also renamed `set_is_playing_and_request_beat_at_time()` to `set_playing_and_request_beat_at_time()`.

3. [x] **`SessionState` is `Send` but not `Sync`** — Verified intentional: C API doesn't document thread-safety for concurrent reads, and session state is designed for local/scoped use. Added comment explaining why `Sync` is not implemented.

4. [x] **Callback API** — Reviewed RAII guard pattern. Decided to keep current design: callbacks are stored in `Link` and automatically cleaned up on drop. Explicit `clear_*` methods provide optional control. Guard pattern would add lifetime complexity without significant benefit since callbacks already have RAII semantics at the `Link` level.

5. [x] **`commit_app_session_state` takes `&mut self`** — Intentional: prevents calling while `AudioLink` exists (which holds `&mut Link`). This uses the borrow checker to enforce the C API's recommendation against concurrent audio/app thread session state modifications.

6. [x] **Missing `play()`/`stop()` convenience methods on `SessionState`** — Similar to the `enable()`/`disable()`/`set_enabled()` pattern we just added.

7. [ ] **`set_playing_and_request_beat_at_time` and similar combined methods** — Review whether these convenience methods pulling in `Link` are the right API design, or if users should compose the operations themselves.

8. [ ] **Consider `SessionState::commit()` method** — Currently users must call `Link::commit_app_session_state(&state)` or `AudioLink::commit_session_state(&state)`. Consider whether `SessionState` should have a `commit()` method that takes a reference to `Link`/`AudioLink`, trading the current explicit pattern for convenience.

9. [ ] **Update naming differences table** — Keep the "Naming Differences from the C/C++ API" section in `lib.rs` module docs up to date as the API evolves.
