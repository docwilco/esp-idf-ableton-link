# Contributing

## Development Setup

After cloning the repository, configure git to use the project's hooks:

```bash
git config core.hooksPath .githooks
```

This enables the pre-commit hook that automatically regenerates `README.md` from the crate documentation.

## Code Style

Run `cargo fmt` before committing:

```bash
cargo fmt
```

## Design Guidelines

This crate aims to provide an ergonomic, safe Rust API for Ableton Link on embedded systems. When contributing, please keep these goals in mind:

### Ergonomics

- Prefer idiomatic Rust patterns over mirroring the C/C++ API directly
- Use strong types (newtypes, enums) to prevent misuse at compile time
- Provide convenience methods for common operations
- Use builder patterns or default values where appropriate

### Embedded Performance

- Use `LinkTime` and `LinkDuration` instead of `std::time::Duration` (which uses `u128` internally and is expensive on 32-bit embedded targets)
- Minimize allocations; prefer stack allocation where possible
- Avoid unnecessary copies or clones
- Be mindful of code size and runtime overhead
- Document any operations that allocate or have non-trivial cost

### Naming Clarity

- Choose names that are clearer than the original Ableton Link API where possible
- Use consistent terminology (e.g., "transport" instead of "start/stop")
- Prefer explicit names over abbreviations
- Document any terminology that might be unfamiliar

## README Generation

The `README.md` is generated from `src/lib.rs` documentation using [cargo-readme](https://crates.io/crates/cargo-readme). Do not edit `README.md` directly — your changes will be overwritten.

To regenerate manually:

```bash
cargo readme -o README.md
```
