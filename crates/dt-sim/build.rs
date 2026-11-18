//! Generate `CONFIG` for the simulator from the committed example config.
//! The simulator never needs real secrets, so template placeholders are fine.

use std::{env, fs, path::PathBuf};

use dt_config::{parse_api, parse_secrets, parse_user, validate, Files, Placeholders};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let files = Files {
        api: "config/api.toml",
        user: "config/user.example.toml",
        secrets: "config/secrets.example.toml",
    };
    let read = |name: &str| {
        let path = root.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let api = parse_api(&read(files.api)).unwrap_or_else(|e| panic!("{e}"));
    let user = parse_user(files.user, &read(files.user)).unwrap_or_else(|e| panic!("{e}"));
    let secrets =
        parse_secrets(files.secrets, &read(files.secrets)).unwrap_or_else(|e| panic!("{e}"));
    if let Err(e) = validate(&api, &user, &secrets, files, Placeholders::Allow) {
        panic!("{e}");
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("config.rs");
    fs::write(out, dt_config::generate(&api, &user, &secrets)).unwrap();
}
