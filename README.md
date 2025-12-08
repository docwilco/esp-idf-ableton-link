# esp-idf-ableton-link

Safe Rust wrapper for [Ableton Link](https://www.ableton.com/en/link/) on ESP32 via ESP-IDF.

[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://docwilco.github.io/esp-idf-ableton-link/)

## Overview

This crate provides a safe Rust API for Ableton Link, enabling musical applications to synchronize tempo and beat phase over a local network on ESP32 hardware.

## Supported Hardware

Currently only Xtensa-based ESP32 chips are supported:

- ESP32
- ESP32-S2
- ESP32-S3

## Documentation

API documentation is available at: https://docwilco.github.io/esp-idf-ableton-link/

## Usage

It is recommended to start your project using the [esp-idf-template](https://github.com/esp-rs/esp-idf-template?tab=readme-ov-file#generate-the-project).

### Required Configuration

This crate requires specific configuration in your ESP32 project. All four steps below are mandatory.

#### 1. Add the crate and build dependency

```sh
cargo add esp-idf-ableton-link
cargo add --build embuild
```

#### 2. Set `ESP_IDF_SYS_ROOT_CRATE` in `.cargo/config.toml`

The `ESP_IDF_SYS_ROOT_CRATE` environment variable must be set so that `esp-idf-sys` can discover the extra component configuration from this crate.

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

Replace `your-firmware-crate-name` with the `name` field from your project's `Cargo.toml`.

For more information, see the [ESP-IDF configuration documentation](https://github.com/esp-rs/esp-idf-sys/blob/master/BUILD-OPTIONS.md#esp-idf-configuration).

#### 3. Enable C++ Exceptions in `sdkconfig.defaults`

Ableton Link requires C++ exception support. Add this to your project's `sdkconfig.defaults`:

```
CONFIG_COMPILER_CXX_EXCEPTIONS=y
```

You may also want to increase the main task stack size (Rust often needs more than the default 3KB):

```
CONFIG_ESP_MAIN_TASK_STACK_SIZE=8000
```

For more information about ESP-IDF sdkconfig options, see the [ESP-IDF KConfig reference](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/kconfig.html).

#### 4. Add `embuild::espidf::sysenv::output()` to your `build.rs`

Your project's `build.rs` must call `embuild::espidf::sysenv::output()` to propagate ESP-IDF configuration flags (such as `esp_idf_compiler_cxx_exceptions`) to the Rust compiler:

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
let now = link.clock_micros();
let beat = state.beat_at_time(now, 4.0); // 4 beats per bar
log::info!("Current beat: {}", beat);
```

## License

GPL-2.0-or-later. See [LICENSE.md](LICENSE.md) for details.
