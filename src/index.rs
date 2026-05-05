use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub mode: String,
    pub hash: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

impl Index {
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading index {}", path.display()))?;

        let mut entries = Vec::new();

        for line in text.lines() {
            let mut parts = line.splitn(3, ' ');

            let Some(mode) = parts.next() else {
                bail!("invalid index line: missing mode");
            };

            let Some(hash) = parts.next() else {
                bail!("invalid index line: missing hash");
            };

            let Some(path) = parts.next() else {
                bail!("invalid index line: missing path");
            };

            entries.push(IndexEntry {
                mode: mode.to_string(),
                hash: hash.to_string(),
                path: PathBuf::from(path),
            });
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Self { entries })
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut out = String::new();

        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        for entry in entries {
            out.push_str(&format!(
                "{} {} {}\n",
                entry.mode,
                entry.hash,
                entry.path.display()
            ));
        }

        std::fs::write(path.as_ref(), out)
            .with_context(|| format!("writing index {}", path.as_ref().display()))?;

        Ok(())
    }

    pub fn add_or_replace(&mut self, entry: IndexEntry) {
        self.entries.retain(|existing| existing.path != entry.path);
        self.entries.push(entry);
        self.entries.sort_by(|a, b| a.path.cmp(&b.path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn entry(path: &str, hash: &str) -> IndexEntry {
        IndexEntry {
            mode: "100644".to_string(),
            hash: hash.to_string(),
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn default_index_is_empty() {
        let index = Index::default();

        assert!(index.entries.is_empty());
    }

    #[test]
    fn read_missing_index_returns_empty_index() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        let index = Index::read(&index_path).unwrap();

        assert!(index.entries.is_empty());
    }

    #[test]
    fn write_creates_index_file() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        let index = Index {
            entries: vec![entry(
                "README.md",
                "ce013625030ba8dba906f756967f9e9ca394464a",
            )],
        };

        index.write(&index_path).unwrap();
        assert!(index_path.is_file());

        let text = std::fs::read_to_string(index_path).unwrap();
        assert_eq!(
            text,
            "100644 ce013625030ba8dba906f756967f9e9ca394464a README.md\n"
        );
    }

    #[test]
    fn read_parses_existing_index_file() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        std::fs::write(
            &index_path,
            "100644 ce013625030ba8dba906f756967f9e9ca394464a README.md\n",
        )
        .unwrap();

        let index = Index::read(&index_path).unwrap();

        assert_eq!(
            index.entries,
            vec![IndexEntry {
                mode: "100644".to_string(),
                hash: "ce013625030ba8dba906f756967f9e9ca394464a".to_string(),
                path: PathBuf::from("README.md"),
            }]
        );
    }

    #[test]
    fn read_supports_paths_with_spaces() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        std::fs::write(
            &index_path,
            "100644 ce013625030ba8dba906f756967f9e9ca394464a my file.txt\n",
        )
        .unwrap();

        let index = Index::read(&index_path).unwrap();

        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].path, PathBuf::from("my file.txt"));
    }

    #[test]
    fn read_sorts_entries_by_path() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        std::fs::write(
            &index_path,
            "\
100644 f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f src/main.rs
100644 ce013625030ba8dba906f756967f9e9ca394464a README.md
",
        )
        .unwrap();

        let index = Index::read(&index_path).unwrap();

        assert_eq!(index.entries[0].path, PathBuf::from("README.md"));
        assert_eq!(index.entries[1].path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn write_sorts_entries_by_path() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        let index = Index {
            entries: vec![
                entry("src/main.rs", "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f"),
                entry("README.md", "ce013625030ba8dba906f756967f9e9ca394464a"),
            ],
        };

        index.write(&index_path).unwrap();

        let text = std::fs::read_to_string(index_path).unwrap();

        assert_eq!(
            text,
            "\
100644 ce013625030ba8dba906f756967f9e9ca394464a README.md
100644 f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f src/main.rs
"
        );
    }

    #[test]
    fn add_or_replace_adds_new_entry() {
        let mut index = Index::default();

        index.add_or_replace(entry(
            "README.md",
            "ce013625030ba8dba906f756967f9e9ca394464a",
        ));

        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].path, PathBuf::from("README.md"));
    }

    #[test]
    fn add_or_replace_replaces_existing_entry_for_same_path() {
        let mut index = Index::default();

        index.add_or_replace(entry(
            "README.md",
            "1111111111111111111111111111111111111111",
        ));

        index.add_or_replace(entry(
            "README.md",
            "2222222222222222222222222222222222222222",
        ));

        assert_eq!(index.entries.len(), 1);
        assert_eq!(
            index.entries[0].hash,
            "2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn add_or_replace_keeps_entries_sorted() {
        let mut index = Index::default();

        index.add_or_replace(entry(
            "src/main.rs",
            "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f",
        ));

        index.add_or_replace(entry(
            "README.md",
            "ce013625030ba8dba906f756967f9e9ca394464a",
        ));

        assert_eq!(index.entries[0].path, PathBuf::from("README.md"));
        assert_eq!(index.entries[1].path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn read_rejects_line_missing_hash() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        std::fs::write(&index_path, "100644\n").unwrap();

        let err = Index::read(&index_path).unwrap_err();

        assert!(err.to_string().contains("missing hash"));
    }

    #[test]
    fn read_rejects_line_missing_path() {
        let temp = tempdir().unwrap();
        let index_path = temp.path().join("index");

        std::fs::write(
            &index_path,
            "100644 ce013625030ba8dba906f756967f9e9ca394464a\n",
        )
        .unwrap();

        let err = Index::read(&index_path).unwrap_err();

        assert!(err.to_string().contains("missing path"));
    }
}
