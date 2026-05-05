use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Repository {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
}

impl Repository {
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let worktree = path.as_ref();

        fs::create_dir_all(worktree)
            .with_context(|| format!("creating worktree {}", worktree.display()))?;

        let worktree = worktree
            .canonicalize()
            .with_context(|| format!("canonicalizing worktree {}", worktree.display()))?;

        let gitdir = worktree.join(".git");

        if gitdir.exists() {
            bail!("repository already exists: {}", gitdir.display());
        }

        fs::create_dir_all(gitdir.join("objects"))
            .with_context(|| format!("creating {}", gitdir.join("objects").display()))?;

        fs::create_dir_all(gitdir.join("refs").join("heads"))
            .with_context(|| format!("creating {}", gitdir.join("refs/heads").display()))?;

        fs::write(gitdir.join("HEAD"), b"ref: refs/heads/main\n")
            .with_context(|| format!("writing {}", gitdir.join("HEAD").display()))?;

        Ok(Self { worktree, gitdir })
    }

    pub fn discover(start: impl AsRef<Path>) -> Result<Self> {
        let mut current = start.as_ref().canonicalize()?;

        loop {
            let gitdir = current.join(".git");

            if gitdir.is_dir() {
                return Ok(Self {
                    worktree: current,
                    gitdir,
                });
            }

            if !current.pop() {
                bail!("not inside a git repository");
            }
        }
    }

    pub fn object_path(&self, hash: &str) -> Result<PathBuf> {
        if hash.len() != 40 {
            bail!("object hash must be 40 hex characters");
        }

        let dir = &hash[0..2];
        let file = &hash[2..];

        Ok(self.gitdir.join("objects").join(dir).join(file))
    }

    pub fn index_path(&self) -> PathBuf {
        self.gitdir.join("index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_git_directory_structure() {
        let temp = tempdir().unwrap();

        let repo = Repository::init(temp.path()).unwrap();

        assert!(repo.gitdir.exists());
        assert!(repo.gitdir.join("objects").is_dir());
        assert!(repo.gitdir.join("refs").join("heads").is_dir());
        assert!(repo.gitdir.join("HEAD").is_file());
    }

    #[test]
    fn init_writes_head_pointing_to_main() {
        let temp = tempdir().unwrap();

        let repo = Repository::init(temp.path()).unwrap();

        let head = std::fs::read_to_string(repo.gitdir.join("HEAD")).unwrap();

        assert_eq!(head, "ref: refs/heads/main\n");
    }

    #[test]
    fn init_fails_if_repository_already_exists() {
        let temp = tempdir().unwrap();

        Repository::init(temp.path()).unwrap();

        let err = Repository::init(temp.path()).unwrap_err();

        assert!(err.to_string().contains("repository already exists"));
    }

    #[test]
    fn discover_finds_repo_from_worktree_root() {
        let temp = tempdir().unwrap();

        let initialized = Repository::init(temp.path()).unwrap();
        let discovered = Repository::discover(temp.path()).unwrap();

        assert_eq!(discovered.gitdir, initialized.gitdir);
        assert_eq!(discovered.worktree, initialized.worktree);
    }

    #[test]
    fn discover_finds_repo_from_nested_directory() {
        let temp = tempdir().unwrap();

        let initialized = Repository::init(temp.path()).unwrap();

        let nested = temp.path().join("src").join("nested").join("deep");
        std::fs::create_dir_all(&nested).unwrap();

        let discovered = Repository::discover(&nested).unwrap();

        assert_eq!(discovered.gitdir, initialized.gitdir);
        assert_eq!(discovered.worktree, initialized.worktree);
    }

    #[test]
    fn discover_fails_outside_repository() {
        let temp = tempdir().unwrap();

        let err = Repository::discover(temp.path()).unwrap_err();

        assert!(err.to_string().contains("not inside a git repository"));
    }

    #[test]
    fn object_path_splits_hash_into_directory_and_filename() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let hash = "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f";
        let path = repo.object_path(hash).unwrap();

        assert_eq!(
            path,
            repo.gitdir
                .join("objects")
                .join("f2")
                .join("ba8f84ab5c1bce84a7b441cb1959cfc7093b7f")
        );
    }

    #[test]
    fn object_path_rejects_short_hash() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let err = repo.object_path("abc").unwrap_err();

        assert!(err.to_string().contains("object hash must be 40"));
    }
}
