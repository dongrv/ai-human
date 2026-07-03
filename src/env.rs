use std::path::Path;

pub fn load_project_env(project_root: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = project_root.as_ref().join(".env");
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || std::env::var_os(key).is_some() {
            continue;
        }

        std::env::set_var(key, unquote_env_value(value.trim()));
    }

    Ok(())
}

fn unquote_env_value(value: &str) -> &str {
    if value.len() < 2 {
        return value;
    }

    let bytes = value.as_bytes();
    let is_double_quoted = bytes.first() == Some(&b'"') && bytes.last() == Some(&b'"');
    let is_single_quoted = bytes.first() == Some(&b'\'') && bytes.last() == Some(&b'\'');

    if is_double_quoted || is_single_quoted {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use assert_fs::prelude::*;

    use super::load_project_env;

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn loads_project_env_without_overwriting_existing_vars() {
        let _lock = env_lock().lock().unwrap();
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child(".env")
            .write_str(
                r#"
# comment
AI_HUMAN_TEST_ENV_FILE=from-file
AI_HUMAN_TEST_QUOTED="quoted value"
AI_HUMAN_TEST_EXISTING=from-file
"#,
            )
            .unwrap();
        let _env = EnvGuard::set_all([
            ("AI_HUMAN_TEST_ENV_FILE", None),
            ("AI_HUMAN_TEST_QUOTED", None),
            ("AI_HUMAN_TEST_EXISTING", Some("from-env")),
        ]);

        load_project_env(temp.path()).unwrap();

        assert_eq!(
            std::env::var("AI_HUMAN_TEST_ENV_FILE").unwrap(),
            "from-file"
        );
        assert_eq!(
            std::env::var("AI_HUMAN_TEST_QUOTED").unwrap(),
            "quoted value"
        );
        assert_eq!(std::env::var("AI_HUMAN_TEST_EXISTING").unwrap(), "from-env");
    }

    struct EnvGuard {
        originals: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn set_all<const N: usize>(vars: [(&'static str, Option<&str>); N]) -> Self {
            let originals = vars
                .iter()
                .map(|(key, _)| (*key, std::env::var(key).ok()))
                .collect();
            for (key, value) in vars {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
            Self { originals }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.originals.drain(..) {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}
