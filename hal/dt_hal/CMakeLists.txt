# Desktop terminal hardware abstraction layer (ESP-IDF component "dt_hal").
# Built by esp-idf-sys as an extra component; see firmware/dt-hal/Cargo.toml.
idf_component_register(
    SRCS
        "src/epd_transport.c"
        "src/hal_err.c"
        "src/hal_version.c"
    INCLUDE_DIRS
        "include"
    PRIV_REQUIRES
        driver
        esp_driver_gpio
        esp_driver_spi
)
