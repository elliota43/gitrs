use crate::object::Object;
use crate::repository::Repository;

use anyhow::{Context, Result};
use std::fs;

#[derive(Debug, Clone)]
pub struct ObjectDatabase {
    repo: Repository,
}

impl ObjectDatabase {
    pub fn new(repo: Repository) -> Self {
        Self { repo }
    }

    pub fn write(&self, object: &Object) -> Result<String> {
        let hash = object.hash();
        let path = self.repo.object_path(&hash)?;

        if path.exists() {
            return Ok(hash);
        }

        let dir = path
            .parent()
            .context("object path unexpectedly has no parent dir")?;

        fs::create_dir_all(dir)
            .with_context(|| format!("creating object directory {}", dir.display()))?;

        let compressed = object.compress()?;

        fs::write(&path, compressed)
            .with_context(|| format!("writing object {}", path.display()))?;

        Ok(hash)
    }

    pub fn read(&self, hash: &str) -> Result<Object> {
        let path = self.repo.object_path(hash)?;

        let compressed =
            fs::read(&path).with_context(|| format!("reading object {}", path.display()))?;

        Object::decompress(&compressed)
    }

    pub fn exists(&self, hash: &str) -> Result<bool> {
        let path = self.repo.object_path(hash)?;
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_stores_object_at_expected_path() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo.clone());

        let object = Object::blob(b"abc".to_vec());
        let hash = database.write(&object).unwrap();

        let path = repo.object_path(&hash).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn write_returns_git_compatible_hash() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let object = Object::blob(b"abc".to_vec());
        let hash = database.write(&object).unwrap();

        assert_eq!(hash, "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f");
    }

    #[test]
    fn read_returns_written_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let object = Object::blob(b"hello database".to_vec());
        let hash = database.write(&object).unwrap();

        let read_back = database.read(&hash).unwrap();

        assert_eq!(read_back.kind, object.kind);
        assert_eq!(read_back.data, object.data);
        assert_eq!(read_back.hash(), object.hash());
    }

    #[test]
    fn exists_returns_true_for_written_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let object = Object::blob(b"abc".to_vec());
        let hash = database.write(&object).unwrap();

        assert!(database.exists(&hash).unwrap());
    }

    #[test]
    fn exists_returns_false_for_missing_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let hash = "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f";

        assert!(!database.exists(hash).unwrap());
    }

    #[test]
    fn read_fails_for_missing_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let hash = "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f";

        let err = database.read(hash).unwrap_err();

        assert!(err.to_string().contains("reading object"));
    }

    #[test]
    fn write_is_idempotent() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();
        let database = ObjectDatabase::new(repo);

        let object = Object::blob(b"abc".to_vec());
        let hash1 = database.write(&object).unwrap();
        let hash2 = database.write(&object).unwrap();

        assert_eq!(hash1, hash2);

        let read_back = database.read(&hash1).unwrap();

        assert_eq!(read_back.data, b"abc");
    }
}
