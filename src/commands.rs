use crate::database::ObjectDatabase;
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
    let data = read_hash_object_input(path, stdin_mode)?;

    let object = Object::blob(data);
    let hash = object.hash();

    if write {
        let repo = Repository::discover(".")?;
        let database = ObjectDatabase::new(repo);

        database.write(&object)?;
    }

    println!("{hash}");

    Ok(())
}

pub fn cat_file(mode: &str, hash: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let database = ObjectDatabase::new(repo);

    let object = database.read(hash)?;

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

fn read_hash_object_input(path: Option<&str>, stdin_mode: bool) -> Result<Vec<u8>> {
    if stdin_mode {
        return read_stdin();
    }

    let Some(path) = path else {
        bail!("missing path; use `hash-object <path>` or `hash-object --stdin`");
    };

    fs::read(path).with_context(|| format!("reading {path}"))
}

fn read_stdin() -> Result<Vec<u8>> {
    let mut data = Vec::new();
    io::stdin().read_to_end(&mut data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn read_hash_object_input_reads_file_contents() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("hello.txt");

        std::fs::write(&file, b"hello world").unwrap();

        let data = read_hash_object_input(Some(file.to_str().unwrap()), false).unwrap();

        assert_eq!(data, b"hello world");
    }

    #[test]
    fn read_hash_object_input_requires_path_when_not_stdin() {
        let err = read_hash_object_input(None, false).unwrap_err();

        assert!(err.to_string().contains("missing path"));
    }
}
