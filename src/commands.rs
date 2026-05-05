use crate::object::Object;
use crate::repository::Repository;

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub fn init(path: &str) -> Result<()> {
    let repo = Repository::init(path)?;

    println!(
        "Initialized empty git repository in {}",
        repo.gitdir.display()
    );

    Ok(())
}

pub fn hash_object(path: Option<&str>, stdin_mode: bool, write: bool) -> Result<()> {
    let data = if stdin_mode {
        read_stdin()?
    } else {
        let Some(path) = path else {
            bail!("missing path; use `hash-object <path>` or `hash-object --stdin`");
        };

        fs::read(path).with_context(|| format!("reading {path}"))?
    };

    let object = Object::blob(data);
    let hash = object.hash();

    if write {
        let repo = Repository::discover(".")?;
        write_object(&repo, &object, &hash)?;
    }

    println!("{hash}");

    Ok(())
}

pub fn cat_file(mode: &str, hash: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let object = read_object(&repo, hash)?;

    match mode {
        "-p" => {
            print!("{}", String::from_utf8_lossy(&object.data));
        }

        "-t" => {
            println!("{}", object.kind.as_str());
        }

        "-s" => {
            println!("{}", object.data.len());
        }

        _ => {
            bail!("unsupported cat-file mode: {mode}; expected -p, -t, or -s");
        }
    }
    Ok(())
}

pub fn write_object(repo: &Repository, object: &Object, hash: &str) -> Result<()> {
    let path = repo.object_path(hash)?;
    let dir = path
        .parent()
        .context("object path unexpectedly has no parent directory")?;

    if path.exists() {
        return Ok(());
    }

    fs::create_dir_all(dir)
        .with_context(|| format!("creating object directory {}", dir.display()))?;

    let compressed = object.compress()?;

    fs::write(&path, compressed).with_context(|| format!("writing object {}", path.display()))?;

    Ok(())
}

pub fn read_object(repo: &Repository, hash: &str) -> Result<Object> {
    let path = repo.object_path(hash)?;

    let compressed =
        fs::read(&path).with_context(|| format!("reading object {}", path.display()))?;

    Object::decompress(&compressed)
}

fn read_stdin() -> Result<Vec<u8>> {
    let mut data = Vec::new();
    io::stdin().read_to_end(&mut data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Object;
    use tempfile::tempdir;

    #[test]
    fn init_command_creates_repository() {
        let temp = tempdir().unwrap();

        init(temp.path().to_str().unwrap()).unwrap();

        assert!(temp.path().join(".git").is_dir());
        assert!(temp.path().join(".git").join("objects").is_dir());
        assert!(temp.path().join(".git").join("refs").join("heads").is_dir());
        assert!(temp.path().join(".git").join("HEAD").is_file());
    }

    #[test]
    fn write_object_stores_compressed_object_at_git_object_path() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let object = Object::blob(b"abc".to_vec());
        let hash = object.hash();

        write_object(&repo, &object, &hash).unwrap();
        let path = repo.object_path(&hash).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn read_object_reads_back_written_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let original = Object::blob(b"abc".to_vec());
        let hash = original.hash();

        write_object(&repo, &original, &hash).unwrap();

        let read_back = read_object(&repo, &hash).unwrap();

        assert_eq!(read_back.kind, original.kind);
        assert_eq!(read_back.data, original.data);
        assert_eq!(read_back.hash(), original.hash());
    }

    #[test]
    fn read_object_fails_for_missing_object() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let hash = "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f";
        let err = read_object(&repo, hash).unwrap_err();

        assert!(err.to_string().contains("reading object"));
    }

    #[test]
    fn write_object_is_idempotent() {
        let temp = tempdir().unwrap();
        let repo = Repository::init(temp.path()).unwrap();

        let object = Object::blob(b"abc".to_vec());
        let hash = object.hash();

        write_object(&repo, &object, &hash).unwrap();
        write_object(&repo, &object, &hash).unwrap();

        let read_back = read_object(&repo, &hash).unwrap();

        assert_eq!(read_back.data, b"abc");
    }
}
