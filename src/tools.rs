pub mod fs {
    use std::path::{Component, Path, PathBuf};

    use anyhow::{bail, Context, Result};
    use tokio::fs::{self, OpenOptions};
    use tokio::io::AsyncWriteExt;

    #[derive(Debug, Clone)]
    pub struct ProjectFs {
        project_root: PathBuf,
    }

    impl ProjectFs {
        pub fn new(project_root: impl Into<PathBuf>) -> Self {
            Self {
                project_root: project_root.into(),
            }
        }

        pub async fn read_text(&self, requested_path: &Path) -> Result<ProjectFile> {
            let canonical_root = self.canonical_root().await?;
            let path = if requested_path.is_absolute() {
                requested_path.to_path_buf()
            } else {
                self.project_root.join(requested_path)
            };

            let metadata = fs::symlink_metadata(&path).await.with_context(|| {
                format!(
                    "path must reference an existing project file: {}",
                    requested_path.display()
                )
            })?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() || !file_type.is_file() {
                bail!(
                    "path must reference a regular project file: {}",
                    requested_path.display()
                );
            }

            let canonical_path = fs::canonicalize(&path).await.with_context(|| {
                format!(
                    "path must reference an existing project file: {}",
                    requested_path.display()
                )
            })?;
            if !canonical_path.starts_with(&canonical_root) {
                bail!(
                    "path must stay inside project root: {}. Use --project-root to choose the intended workspace.",
                    requested_path.display()
                );
            }

            let display_path = display_path(&canonical_root, &canonical_path);
            let text = fs::read_to_string(&canonical_path)
                .await
                .with_context(|| format!("path must be readable UTF-8 text: {display_path}"))?;

            Ok(ProjectFile { display_path, text })
        }

        pub async fn append_text(&self, relative_path: &Path, contents: &str) -> Result<String> {
            validate_safe_relative_path(relative_path)?;

            let canonical_root = self.canonical_root().await?;
            let path = self.project_root.join(relative_path);
            let parent = path
                .parent()
                .with_context(|| format!("path has no parent: {}", relative_path.display()))?;

            fs::create_dir_all(parent).await?;

            if fs::try_exists(&path).await? {
                let metadata = fs::symlink_metadata(&path).await?;
                let file_type = metadata.file_type();
                if file_type.is_symlink() || !file_type.is_file() {
                    bail!(
                        "path must reference a regular project file for append: {}",
                        relative_path.display()
                    );
                }
                let canonical_path = fs::canonicalize(&path).await?;
                if !canonical_path.starts_with(&canonical_root) {
                    bail!(
                        "path must stay inside project root: {}",
                        relative_path.display()
                    );
                }
            } else {
                let canonical_parent = fs::canonicalize(parent).await?;
                if !canonical_parent.starts_with(&canonical_root) {
                    bail!(
                        "path must stay inside project root: {}",
                        relative_path.display()
                    );
                }
            }

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            file.write_all(contents.as_bytes()).await?;
            file.flush().await?;

            Ok(relative_path.to_string_lossy().replace('\\', "/"))
        }

        async fn canonical_root(&self) -> Result<PathBuf> {
            fs::create_dir_all(&self.project_root).await?;
            fs::canonicalize(&self.project_root).await.with_context(|| {
                format!(
                    "project root does not exist: {}",
                    self.project_root.display()
                )
            })
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProjectFile {
        pub display_path: String,
        pub text: String,
    }

    fn validate_safe_relative_path(path: &Path) -> Result<()> {
        if path.as_os_str().is_empty() {
            bail!("path must not be empty");
        }
        if path.is_absolute() {
            bail!("path must be relative to project root: {}", path.display());
        }
        for component in path.components() {
            match component {
                Component::Normal(_) => {}
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    bail!("path must stay inside project root: {}", path.display());
                }
            }
        }

        Ok(())
    }

    fn display_path(project_root: &Path, path: &Path) -> String {
        path.strip_prefix(project_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}
