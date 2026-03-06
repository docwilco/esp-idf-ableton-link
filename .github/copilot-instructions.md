# Copilot Instructions

## README Generation

`README.md` is generated from `README.tpl` using `cargo readme`. The template
pulls crate-level doc comments from `src/lib.rs` via `{{readme}}`. Never edit
`README.md` directly — edit the doc comments in `src/lib.rs` instead, then
regenerate with `cargo readme > README.md`.

## API Review TODO

Work through these items one by one:

1. [x] **`i64`/`u64` time inconsistency** — Introduced `Instant` newtype wrapping `i64` microseconds. Renamed `clock_micros()` to `clock_now()`. All time parameters/returns now use `Instant`. Supports `Duration` arithmetic.

2. [x] **`set_is_playing` naming** — Added `play()`/`stop()` convenience methods and renamed `set_is_playing()` to `set_transport_state_at()`. Also renamed `set_is_playing_and_request_beat_at_time()` to `start_transport_and_request_beat_at()`.

3. [x] **`SessionState` is `Send` but not `Sync`** — Verified intentional: C API doesn't document thread-safety for concurrent reads, and session state is designed for local/scoped use. Added comment explaining why `Sync` is not implemented.

4. [x] **Callback API** — Reviewed RAII guard pattern. Decided to keep current design: callbacks are stored in `Link` and automatically cleaned up on drop. Explicit `clear_*` methods provide optional control. Guard pattern would add lifetime complexity without significant benefit since callbacks already have RAII semantics at the `Link` level.

5. [x] **`commit_app_session_state` takes `&mut self`** — Intentional: prevents calling while `RealtimeHandle` exists (which holds `&mut Link`). This uses the borrow checker to enforce the C API's recommendation against concurrent audio/app thread session state modifications.

6. [x] **Missing `play()`/`stop()` convenience methods on `SessionState`** — Similar to the `enable()`/`disable()`/`set_enabled()` pattern we just added.

7. [ ] **`start_transport_and_request_beat_at` and similar combined methods** — Review whether these convenience methods are the right API design, or if users should compose the operations themselves. Also applies to `request_beat_at_transport_state_time`.

8. [ ] **Consider `SessionState::commit()` method** — Currently users must call `Link::commit_app_session_state(&state)` or `RealtimeHandle::commit_session_state(&state)`. Consider whether `SessionState` should have a `commit()` method that takes a reference to `Link`/`RealtimeHandle`, trading the current explicit pattern for convenience.

9. [x] **Update naming differences table** — Keep the "Naming Differences from the C/C++ API" section in `lib.rs` module docs up to date as the API evolves. Updated for 0.2.0-beta.1: added `RealtimeHandle`/`bind_realtime` rows, updated `Instant` C API column from `int64_t / uint64_t` to `int64_t`.
