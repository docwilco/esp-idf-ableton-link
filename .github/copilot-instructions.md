# ESP-IDF Ableton Link

## Project Overview

This project provides safe Rust bindings for Ableton Link on ESP32 hardware using the ESP-IDF framework. It uses esp-idf-sys to build an ESP-IDF extra component, which consists of the `abl_link` C wrapper around the Ableton Link C++ API.

## Architecture

### Project Structure
  
ESP-IDF component wrapper for Ableton Link
- Contains Ableton Link as git submodule at `ableton-link/`
- Uses provided C wrapper around C++ Link API at `ableton-link/extensions/abl_link/`
- Does NOT use any pre-existing Link ports or wrappers outside the Ableton Link repository

### Git Submodule Configuration

The Ableton Link submodule has two remotes:
- `upstream`: Official Ableton repository (git@github.com:Ableton/link.git)
- `origin`: Fork repository (git@github.com:docwilco/ableton-link.git)

## Development Environment

- ESP-IDF environment setup: `/home/drwilco/export-esp.sh`
- Main branch: `main`
- ESP-IDF extra components documentation: https://github.com/esp-rs/esp-idf-sys/blob/master/BUILD-OPTIONS.md#extra-esp-idf-components
- ESP-IDF build system: https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-guides/build-system.html

## Code Conventions

### Rust
- Edition 2021
- Follow standard Rust naming conventions
- Use `#![no_std]` where appropriate for embedded contexts
- Profile settings optimized for embedded

## Important Constraints

1. **Do not use existing Link ports**: All existing Rust or embedded Link attempts have been evaluated and rejected
3. **Ask before expanding**: Any API additions or major architectural changes require approval
4. **No std in sys crate if possible**: esp-idf-ableton-link uses `#![no_std]`

## Testing & Validation

- This crate can only be built as a dependency
- Link submodule stays synchronized with upstream

## Git Workflow

- Commit after completing logical units of work (e.g., after implementing a feature, fixing a bug, or completing a refactor)
- Use conventional commit format when appropriate (feat:, fix:, refactor:, etc.)
- Keep commits focused on single concerns
- Do NOT commit after every single line change - batch related changes together into meaningful commits

## Rust Coding Guidelines

- Follow Rust API design guidelines: https://rust-lang.github.io/api-guidelines/
- Use idiomatic Rust patterns and practices
- After succesful builds, run `cargo clippy` to ensure code quality. Also use pedantic lints.
- Do not just allow lints; either fix the issue using the suggestions, lookup the lint documentation, or discuss if unsure. A very good justification comment is the last resort.
- Use Rustfmt for consistent code formatting. Run `cargo fmt` before committing code.
- Write documentation comments for public APIs using `///`
- esp-idf-sys and related crates are used for ESP32 bindings: Follow their conventions where applicable.
- Use `esp-idf-svc` for higher-level services like networking.
- Use ESP-IDF logging macros for logging (e.g., `info!`, `warn!`, `error!`). Set max log level to Debug for development. Set default log level to Info.
- esp-idf-sys uses embuild-rs for build configuration: Refer to its documentation for custom build settings. And make sure to properly use it for building our C++ wrapper and esp-idf-ableton-link crate.
- esp-idf-ableton-link is built as an ESP-IDF extra component using the `package.metadata.esp-idf-sys.extra_components` mechanism

## Project TODOs

- Fixup commit messages
- Split out the ESP-IDF component
