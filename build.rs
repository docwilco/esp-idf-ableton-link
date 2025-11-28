fn main() {
    // Propagate cfgs from esp-idf-sys (e.g., esp_idf_compiler_cxx_exceptions)
    embuild::espidf::sysenv::output();

    println!("cargo:rerun-if-env-changed=ESP_IDF_SYS_ROOT_CRATE");

    if std::env::var("ESP_IDF_SYS_ROOT_CRATE").is_err() {
        panic!(
            r#"
================================================================================
                    ESP-IDF ABLETON LINK CONFIGURATION ERROR
================================================================================

The ESP_IDF_SYS_ROOT_CRATE environment variable is not set.

This variable is REQUIRED for esp-idf-ableton-link to work correctly.

TO FIX THIS:
------------
Add the following to your project's `.cargo/config.toml`:

    [env]
    ESP_IDF_SYS_ROOT_CRATE = "your-firmware-crate-name"

Replace "your-firmware-crate-name" with the actual name of your root crate
(the `name` field in your Cargo.toml's [package] section).

WHY THIS IS REQUIRED:
---------------------
esp-idf-sys uses this variable to identify the root crate. It then reads the
root crate's Cargo.toml metadata AND scans its direct dependencies (including
esp-idf-ableton-link) for extra component configurations.

Without this variable, esp-idf-sys cannot discover our extra_components
configuration, and the Ableton Link bindings will not be generated.

EXAMPLE .cargo/config.toml:
---------------------------
    [build]
    target = "xtensa-esp32-espidf"

    [target.xtensa-esp32-espidf]
    linker = "ldproxy"
    runner = "espflash flash --monitor"
    rustflags = [ "--cfg",  "espidf_time64"]

    [env]
    MCU="esp32"
    ESP_IDF_VERSION = "v5.3.3"
    ESP_IDF_SYS_ROOT_CRATE = "my-esp32-project"


For more information, see:
https://github.com/esp-rs/esp-idf-sys/blob/master/BUILD-OPTIONS.md#extra-esp-idf-components

================================================================================
"#
        );
    }

    // Check that C++ exceptions are enabled (required for Ableton Link)
    if let Some(cfg_args) = embuild::espidf::sysenv::cfg_args() {
        if cfg_args.get("esp_idf_compiler_cxx_exceptions").is_none() {
            panic!(
                r#"
================================================================================
                    ESP-IDF ABLETON LINK CONFIGURATION ERROR
================================================================================

Ableton Link requires C++ exception support, but CONFIG_COMPILER_CXX_EXCEPTIONS
is not enabled in your ESP-IDF configuration.

To fix this, add the following line to your project's `sdkconfig.defaults` file:

    CONFIG_COMPILER_CXX_EXCEPTIONS=y

Then clean and rebuild:

    cargo clean
    cargo build

WHY THIS IS REQUIRED:
---------------------
Ableton Link's C++ implementation uses exceptions for error handling. Without
exception support enabled in ESP-IDF, the Link library will fail to compile.

LOCATION:
---------
Create or edit `sdkconfig.defaults` in your project root (the same directory
as your main Cargo.toml).

For more information about ESP-IDF sdkconfig options, see:
https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/kconfig.html

================================================================================
"#
            );
        }
    }
}
