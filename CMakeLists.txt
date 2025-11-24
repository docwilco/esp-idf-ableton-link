idf_component_register(
    SRCS "src/link_wrapper.cpp"
    INCLUDE_DIRS "include"
    REQUIRES asio
    PRIV_REQUIRES sock_utils esp_timer esp_driver_gptimer esp_netif
)

# C++11 is required for Ableton Link
target_compile_features(${COMPONENT_LIB} PUBLIC cxx_std_11)

# Ableton Link include directories (internal, not exposed to consumers)
target_include_directories(${COMPONENT_LIB} PRIVATE
    "ableton-link/include"
)

# ESP_PLATFORM is automatically defined by ESP-IDF, which Link uses to detect ESP32
# No need for LINK_PLATFORM_LINUX - Link has native ESP32 support
