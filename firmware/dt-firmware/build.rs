use std::{env, path::PathBuf};

fn main() {
    // Pass ESP-IDF build settings (linker args, cfgs) from esp-idf-sys to rustc.
    embuild::espidf::sysenv::output();

    // Validate config/*.toml, print the derived API schedule and generate
    // `CONFIG` into OUT_DIR. Stops the build with explanations on any problem.
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    dt_config::run_build_script(&root);
}
