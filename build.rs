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

This variable is REQUIRED for esp-idf-ableton-link to work correctly. Without
it, esp-idf-sys cannot locate the extra component configuration needed to build
the Ableton Link bindings.

TO FIX THIS:
------------
Add the following to your project's `.cargo/config.toml`:

    [env]
    ESP_IDF_SYS_ROOT_CRATE = "your-firmware-crate-name"

Replace "your-firmware-crate-name" with the actual name of your root crate
(the `name` field in your Cargo.toml's [package] section).

WHY THIS IS REQUIRED:
---------------------
esp-idf-ableton-link provides an ESP-IDF extra component (abl_link) that
esp-idf-sys then generates bindings for. Without this environment variable,
the bindings will not be generated and compilation will fail.

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
}
