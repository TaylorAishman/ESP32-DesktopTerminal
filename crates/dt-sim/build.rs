//! Generate `CONFIG` for the simulator from the committed example config.
//! The simulator never needs real secrets.

use std::{env, path::PathBuf};

fn main() {
    // The simulator always uses the templates.
    env::set_var(dt_config::load::ALLOW_EXAMPLE_ENV, "1");
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    dt_config::run_build_script(&root);
}
