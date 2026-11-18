//! The whole build-time pipeline: read, parse, validate, derive the schedule
//! and generate code. Used by the firmware's and the simulator's build scripts.

use std::fs;
use std::path::{Path, PathBuf};

use crate::api::{parse_api, API_FILE};
use crate::error::{ConfigError, ConfigErrors};
use crate::schedule::{check_staleness, derive_schedule, render_table};
use crate::secrets::{parse_secrets, SECRETS_EXAMPLE_FILE, SECRETS_FILE};
use crate::user::{parse_user, USER_EXAMPLE_FILE, USER_FILE};
use crate::validate::{validate, Files, Placeholders};

/// Environment variable that makes the build use the committed templates
/// instead of config/user.toml and config/secrets.toml (for CI).
pub const ALLOW_EXAMPLE_ENV: &str = "DT_ALLOW_EXAMPLE_CONFIG";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// config/user.toml and config/secrets.toml, which must exist and be
    /// filled in.
    Personal,
    /// The *.example.toml templates, placeholders allowed.
    Examples,
}

impl Source {
    /// `Examples` if `DT_ALLOW_EXAMPLE_CONFIG` is set to a non-empty value
    /// other than "0".
    pub fn from_env() -> Source {
        match std::env::var(ALLOW_EXAMPLE_ENV) {
            Ok(v) if !v.is_empty() && v != "0" => Source::Examples,
            _ => Source::Personal,
        }
    }
}

/// Result of a successful load.
#[derive(Debug)]
pub struct Loaded {
    /// Rust source for `CONFIG`. Contains secrets: write it to OUT_DIR only.
    pub source: String,
    /// The derived schedule, for the build log.
    pub schedule_table: String,
    /// Files read, for `cargo:rerun-if-changed`.
    pub inputs: Vec<PathBuf>,
}

/// Load, check and generate the configuration under `root` (the repository).
pub fn load(root: &Path, source: Source) -> Result<Loaded, ConfigErrors> {
    let files = match source {
        Source::Personal => Files {
            api: API_FILE,
            user: USER_FILE,
            secrets: SECRETS_FILE,
        },
        Source::Examples => Files {
            api: API_FILE,
            user: USER_EXAMPLE_FILE,
            secrets: SECRETS_EXAMPLE_FILE,
        },
    };
    let inputs: Vec<PathBuf> = [files.api, files.user, files.secrets]
        .iter()
        .map(|f| root.join(f))
        .collect();

    let read = |name: &str, template: &str| -> Result<String, ConfigError> {
        fs::read_to_string(root.join(name)).map_err(|e| {
            let err = ConfigError::new(name, format!("cannot be read: {e}"));
            if template.is_empty() {
                err
            } else {
                err.fix(format!(
                    "Copy {template} to {name} and fill it in. To build with the templates instead (no Wi-Fi or data), set {ALLOW_EXAMPLE_ENV}=1."
                ))
            }
        })
    };
    let one = |e: ConfigError| ConfigErrors(vec![e]);

    let api_text = read(files.api, "").map_err(one)?;
    let (user_template, secrets_template) = match source {
        Source::Personal => (USER_EXAMPLE_FILE, SECRETS_EXAMPLE_FILE),
        Source::Examples => ("", ""),
    };
    let mut errors = Vec::new();
    let user_text = read(files.user, user_template)
        .map_err(|e| errors.push(e))
        .ok();
    let secrets_text = read(files.secrets, secrets_template)
        .map_err(|e| errors.push(e))
        .ok();
    if !errors.is_empty() {
        return Err(ConfigErrors(errors));
    }
    let (user_text, secrets_text) = (
        user_text.unwrap_or_default(),
        secrets_text.unwrap_or_default(),
    );

    let api = parse_api(&api_text).map_err(one)?;
    let user = parse_user(files.user, &user_text).map_err(one)?;
    let secrets = parse_secrets(files.secrets, &secrets_text).map_err(one)?;

    let placeholders = match source {
        Source::Personal => Placeholders::Reject,
        Source::Examples => Placeholders::Allow,
    };
    validate(&api, &user, &secrets, files, placeholders)?;
    let schedule = derive_schedule(&api, &user);
    check_staleness(&api, &schedule, files.api)?;

    Ok(Loaded {
        source: crate::generate::generate(&api, &user, &secrets),
        schedule_table: render_table(&api, &schedule),
        inputs,
    })
}

/// Everything a build script needs to do: load the config, write
/// `OUT_DIR/config.rs`, print the schedule, and on failure print the
/// problems and exit with an error. `root` is the repository root.
pub fn run_build_script(root: &Path) {
    println!("cargo:rerun-if-env-changed={ALLOW_EXAMPLE_ENV}");
    for f in [
        API_FILE,
        USER_FILE,
        SECRETS_FILE,
        USER_EXAMPLE_FILE,
        SECRETS_EXAMPLE_FILE,
    ] {
        println!("cargo:rerun-if-changed={}", root.join(f).display());
    }
    let source = Source::from_env();
    match load(root, source) {
        Ok(loaded) => {
            let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
            if let Err(e) = fs::write(out.join("config.rs"), &loaded.source) {
                eprintln!("error: cannot write generated config: {e}");
                std::process::exit(1);
            }
            if source == Source::Examples {
                println!("cargo:warning=Building with the example config ({ALLOW_EXAMPLE_ENV} is set): no real Wi-Fi or API keys.");
            }
            for line in loaded.schedule_table.lines() {
                println!("cargo:warning=schedule: {line}");
            }
        }
        Err(errors) => {
            eprintln!("\nConfiguration problems ({}):\n", errors.0.len());
            for e in &errors.0 {
                eprintln!("  {}\n", e.to_string().replace('\n', "\n  "));
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dt-config-load-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("config")).unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for f in [API_FILE, USER_EXAMPLE_FILE, SECRETS_EXAMPLE_FILE] {
            fs::copy(repo.join(f), dir.join(f)).unwrap();
        }
        dir
    }

    #[test]
    fn examples_load_and_generate() {
        let root = temp_root("examples");
        let loaded = load(&root, Source::Examples).unwrap();
        assert!(loaded.source.contains("pub static CONFIG"));
        assert!(loaded.schedule_table.contains("top_headlines"));
    }

    #[test]
    fn missing_personal_files_explain_what_to_do() {
        let root = temp_root("missing");
        let e = load(&root, Source::Personal).unwrap_err().to_string();
        assert!(e.contains("config/secrets.toml: cannot be read"), "{e}");
        assert!(
            e.contains(
                "fix: Copy config/secrets.example.toml to config/secrets.toml and fill it in."
            ),
            "{e}"
        );
        assert!(e.contains("DT_ALLOW_EXAMPLE_CONFIG=1"), "{e}");
        assert!(e.contains("config/user.toml: cannot be read"), "{e}");
    }

    #[test]
    fn personal_placeholders_are_rejected() {
        let root = temp_root("placeholders");
        fs::copy(root.join(USER_EXAMPLE_FILE), root.join(USER_FILE)).unwrap();
        fs::copy(root.join(SECRETS_EXAMPLE_FILE), root.join(SECRETS_FILE)).unwrap();
        let e = load(&root, Source::Personal).unwrap_err().to_string();
        assert!(
            e.contains("api_keys.newsapi: is still the template placeholder"),
            "{e}"
        );
    }
}
