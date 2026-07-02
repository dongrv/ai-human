pub mod loader {
    use std::collections::HashSet;
    use std::io::ErrorKind;
    use std::path::{Component, Path, PathBuf};

    use anyhow::Result;
    use tokio::fs;
    use walkdir::WalkDir;

    const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LoadedContext {
        pub sources: Vec<String>,
        pub combined_text: String,
    }

    #[derive(Debug, Clone)]
    pub struct ContextLoader {
        project_root: PathBuf,
        max_file_bytes: u64,
    }

    impl ContextLoader {
        pub fn new(project_root: PathBuf) -> Self {
            Self {
                project_root,
                max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            }
        }

        pub async fn load_for_input(&self, input: &str) -> Result<LoadedContext> {
            let canonical_root = fs::canonicalize(&self.project_root).await?;
            let mut candidates = vec![
                self.project_root.join("AGENTS.md"),
                self.project_root.join("README.md"),
                self.project_root.join(".agents/README.md"),
                self.project_root.join(".ai-human/knowledge/README.md"),
                self.project_root.join(".ai-human/knowledge/project-map.md"),
                self.project_root
                    .join(".ai-human/knowledge/engineering-rules.md"),
                self.project_root.join(".ai-human/knowledge/faq.md"),
            ];

            candidates.extend(self.knowledge_workflow_files());

            if let Some(path_hint) = first_path_hint(input) {
                candidates.push(self.project_root.join(path_hint));
            }

            let mut seen = HashSet::new();
            let mut sources = Vec::new();
            let mut combined_text = String::new();

            for path in candidates {
                let source = normalize_path(&self.project_root, &path);
                if !seen.insert(source.clone()) {
                    continue;
                }

                let metadata = match fs::symlink_metadata(&path).await {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.into()),
                };

                let file_type = metadata.file_type();
                if file_type.is_symlink()
                    || !file_type.is_file()
                    || metadata.len() > self.max_file_bytes
                {
                    continue;
                }

                let canonical_path = match fs::canonicalize(&path).await {
                    Ok(path) => path,
                    Err(error) if error.kind() == ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.into()),
                };
                if !canonical_path.starts_with(&canonical_root) {
                    continue;
                }

                let Ok(text) = fs::read_to_string(&canonical_path).await else {
                    continue;
                };
                sources.push(source.clone());
                combined_text.push_str(&format!("\n\n--- SOURCE: {source} ---\n{text}\n"));
            }

            Ok(LoadedContext {
                sources,
                combined_text,
            })
        }

        fn knowledge_workflow_files(&self) -> Vec<PathBuf> {
            let dir = self.project_root.join(".ai-human/knowledge/workflows");
            if !dir.exists() {
                return Vec::new();
            }

            let mut paths = WalkDir::new(dir)
                .max_depth(1)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| entry.into_path())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
                })
                .collect::<Vec<_>>();

            paths.sort_by(|left, right| {
                normalize_path(&self.project_root, left)
                    .cmp(&normalize_path(&self.project_root, right))
            });
            paths
        }
    }

    fn first_path_hint(input: &str) -> Option<PathBuf> {
        input.split_whitespace().find_map(|token| {
            let token = token.trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | '`' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            });

            if token.is_empty()
                || !(token.contains('/') || token.contains('\\') || has_file_extension(token))
            {
                return None;
            }

            let path = PathBuf::from(token);
            is_project_relative_path(&path).then_some(path)
        })
    }

    fn has_file_extension(token: &str) -> bool {
        Path::new(token).extension().is_some()
    }

    fn is_project_relative_path(path: &Path) -> bool {
        !path.is_absolute()
            && path
                .components()
                .all(|component| !matches!(component, Component::ParentDir | Component::Prefix(_)))
    }

    fn normalize_path(root: &Path, path: &Path) -> String {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}
