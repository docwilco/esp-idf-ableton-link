idf_component_register(
    SRCS "src/link_wrapper.cpp"
    INCLUDE_DIRS "include"
    REQUIRES asio
    PRIV_REQUIRES sock_utils esp_timer esp_driver_gptimer esp_netif
)

# C++11 is required for Ableton Link - use specific feature like the official example
# Using PRIVATE since only our wrapper needs C++11, not consumers
target_compile_features(${COMPONENT_LIB} PRIVATE cxx_generalized_initializers)

# Ableton Link include directories (internal, not exposed to consumers)
target_include_directories(${COMPONENT_LIB} PRIVATE
    "ableton-link/include"
)

# ESP_PLATFORM is automatically defined by ESP-IDF, which Link uses to detect ESP32
# No need for LINK_PLATFORM_LINUX - Link has native ESP32 support

# Define required ESP32 platform configuration for Ableton Link
# LINK_ESP_TASK_CORE_ID: Which core to run Link tasks on (0 or 1, use 1 to avoid WiFi core)
target_compile_definitions(${COMPONENT_LIB} PRIVATE
    LINK_ESP_TASK_CORE_ID=1
)

# Suppress return-type warnings for Link code (ASIO_NO_EXCEPTIONS causes unreachable code paths)
# Link doesn't fully support no-exceptions mode
# Add -fexceptions explicitly like the official ESP32 example
target_compile_options(${COMPONENT_LIB} PRIVATE
    -Wno-return-type
    -fexceptions
)

# Fix link order issue: libstdc++ needs libc symbols (wctype, wcslen, strtof, etc.)
# but in ESP-IDF+Rust builds, -lc comes BEFORE -lstdc++. Since the linker only makes
# one pass, symbols needed by stdc++ are not found. Adding "c" here ensures libc
# appears again AFTER stdc++ in the link line.
target_link_libraries(${COMPONENT_LIB} PRIVATE c)
