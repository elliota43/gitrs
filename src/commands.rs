use crate::database::ObjectDatabase;
use crate::index::{Index, IndexEntry};
use crate::object::Object;
use crate::repository::Repository;
use crate::tree::{Tree, hash_to_hex};

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

fn kind_for_mode(mode: &str) -> &'static str {
    match mode {
        "40000" => "tree",
        _ => "blob",
    }
}

pub fn ls_tree(hash: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let database = ObjectDatabase::new(repo);

    let object = database.read(hash)?;

    if object.kind != crate::object::ObjectKind::Tree {
        bail!("object {hash} is not a tree");
    }

    let tree = Tree::parse(&object.data)?;

    print!("{}", format_ls_tree(&tree));

    Ok(())
}

pub fn update_index_add(path: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let database = ObjectDatabase::new(repo.clone());

    let full_path = repo.worktree.join(path);
    let data = fs::read(&full_path).with_context(|| format!("reading {}", full_path.display()))?;

    let object = Object::blob(data);
    let hash = database.write(&object)?;

    let mut index = Index::read(repo.index_path())?;

    index.add_or_replace(IndexEntry {
        mode: "100644".to_string(),
        hash,
        path: path.into(),
    });

    index.write(repo.index_path())?;

    Ok(())
}

pub(crate) fn format_ls_tree(tree: &Tree) -> String {
    let mut out = String::new();

    for entry in &tree.entries {
        out.push_str(&format!(
            "{} {} {}\t{}\n",
            entry.mode,
            kind_for_mode(&entry.mode),
            hash_to_hex(&entry.hash),
            entry.name
        ));
    }
    out
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

    #[test]
    fn format_ls_tree_formats_blob_entry() {
        use crate::tree::{Tree, TreeEntry, hex_to_hash};

        let tree = Tree::new(vec![TreeEntry {
            mode: "100644".to_string(),
            name: "hello.txt".to_string(),
            hash: hex_to_hash("f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f").unwrap(),
        }]);

        let out = format_ls_tree(&tree);

        assert_eq!(
            out,
            "100644 blob f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f\thello.txt\n"
        );
    }

    #[test]
    fn format_ls_tree_formats_tree_entry() {
        use crate::tree::{Tree, TreeEntry, hex_to_hash};

        let tree = Tree::new(vec![TreeEntry {
            mode: "40000".to_string(),
            name: "src".to_string(),
            hash: hex_to_hash("a9993e364706816aba3e25717850c26c9cd0d89d").unwrap(),
        }]);

        let out = format_ls_tree(&tree);

        assert_eq!(
            out,
            "40000 tree a9993e364706816aba3e25717850c26c9cd0d89d\tsrc\n"
        );
    }

    #[test]
    fn format_ls_tree_formats_multiple_entries() {
        use crate::tree::{Tree, TreeEntry, hex_to_hash};

        let tree = Tree::new(vec![
            TreeEntry {
                mode: "100644".to_string(),
                name: "README.md".to_string(),
                hash: hex_to_hash("ce013625030ba8dba906f756967f9e9ca394464a").unwrap(),
            },
            TreeEntry {
                mode: "40000".to_string(),
                name: "src".to_string(),
                hash: hex_to_hash("a9993e364706816aba3e25717850c26c9cd0d89d").unwrap(),
            },
        ]);

        let out = format_ls_tree(&tree);

        assert_eq!(
            out,
            "\
100644 blob ce013625030ba8dba906f756967f9e9ca394464a\tREADME.md
40000 tree a9993e364706816aba3e25717850c26c9cd0d89d\tsrc
"
        );
    }
}
