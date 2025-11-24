idf_component_register(
    SRCS "src/link_wrapper.cpp"
    INCLUDE_DIRS "include" "ableton-link/include"
    REQUIRES ""
    PRIV_REQUIRES ""
)

# C++11 is required for Ableton Link
target_compile_features(${COMPONENT_LIB} PUBLIC cxx_std_11)

# Set Link platform definition
target_compile_definitions(${COMPONENT_LIB} PRIVATE LINK_PLATFORM_LINUX=1)
