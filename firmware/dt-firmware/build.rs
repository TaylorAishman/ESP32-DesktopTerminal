fn main() {
    // Pass ESP-IDF build settings (linker args, cfgs) from esp-idf-sys to rustc.
    embuild::espidf::sysenv::output();
}
