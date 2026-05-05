use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub hash: [u8; 20],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self { entries }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();

        for entry in &self.entries {
            out.extend_from_slice(entry.mode.as_bytes());
            out.push(b' ');
            out.extend_from_slice(entry.name.as_bytes());
            out.push(b'\0');
            out.extend_from_slice(&entry.hash);
        }

        out
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let mode_start = pos;

            while pos < data.len() && data[pos] != b' ' {
                pos += 1;
            }

            if pos >= data.len() {
                bail!("invalid tree entry: missing space after mode");
            }

            let mode = std::str::from_utf8(&data[mode_start..pos])
                .context("tree entry mode is not valid utf-8")?
                .to_string();

            pos += 1;

            let name_start = pos;

            while pos < data.len() && data[pos] != b'\0' {
                pos += 1;
            }

            if pos >= data.len() {
                bail!("invalid tree entry: missing NUL after name");
            }

            let name = std::str::from_utf8(&data[name_start..pos])
                .context("tree entry name is not valid utf-8")?
                .to_string();
            pos += 1;

            if pos + 20 > data.len() {
                bail!("invalid tree entry: missing 20-byte object hash");
            }

            let mut hash = [0u8; 20];
            hash.copy_from_slice(&data[pos..pos + 20]);
            pos += 20;

            entries.push(TreeEntry { mode, name, hash });
        }
        Ok(Self { entries })
    }
}

pub fn hash_to_hex(hash: &[u8; 20]) -> String {
    hex::encode(hash)
}

pub fn hex_to_hash(hex_hash: &str) -> Result<[u8; 20]> {
    let bytes = hex::decode(hex_hash).context("invalid hex object hash")?;

    if bytes.len() != 20 {
        bail!("object hash must decode to exactly 20 bytes");
    }

    let mut hash = [0u8; 20];
    hash.copy_from_slice(&bytes);

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hash() -> [u8; 20] {
        hex_to_hash("f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f").unwrap()
    }

    #[test]
    fn hex_to_hash_decodes_40_character_sha1() {
        let hash = hex_to_hash("f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f").unwrap();

        assert_eq!(hash.len(), 20);
        assert_eq!(
            hex::encode(hash),
            "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f"
        );
    }

    #[test]
    fn hex_to_hash_rejects_short_hash() {
        let err = hex_to_hash("abcd").unwrap_err();

        assert!(
            err.to_string()
                .contains("object hash must decode to exactly 20 bytes")
        );
    }

    #[test]
    fn hex_to_hash_rejects_invalid_hex() {
        let err = hex_to_hash("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").unwrap_err();

        assert!(err.to_string().contains("invalid hex object hash"));
    }

    #[test]
    fn hash_to_hex_encodes_raw_hash() {
        let raw = sample_hash();

        let encoded = hash_to_hex(&raw);

        assert_eq!(encoded, "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f");
    }

    #[test]
    fn tree_serializes_single_blob_entry() {
        let tree = Tree::new(vec![TreeEntry {
            mode: "100644".to_string(),
            name: "hello.txt".to_string(),
            hash: sample_hash(),
        }]);
        let serialized = tree.serialize();

        let mut expected = Vec::new();
        expected.extend_from_slice(b"100644 hello.txt\0");
        expected.extend_from_slice(&sample_hash());

        assert_eq!(serialized, expected);
    }

    #[test]
    fn tree_parses_single_blob_entry() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&sample_hash());

        let tree = Tree::parse(&raw).unwrap();

        assert_eq!(
            tree,
            Tree {
                entries: vec![TreeEntry {
                    mode: "100644".to_string(),
                    name: "hello.txt".to_string(),
                    hash: sample_hash(),
                }]
            }
        );
    }

    #[test]
    fn tree_serializes_and_parses_round_trip() {
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

        let serialized = tree.serialize();
        let parsed = Tree::parse(&serialized).unwrap();

        assert_eq!(parsed, tree);
    }

    #[test]
    fn tree_parse_rejects_missing_space_after_mode() {
        let raw = b"100644hello.txt\0";
        let err = Tree::parse(raw).unwrap_err();

        assert!(err.to_string().contains("missing space after mode"));
    }

    #[test]
    fn tree_parse_rejects_missing_nul_after_name() {
        let raw = b"100644 hello.txt";
        let err = Tree::parse(raw).unwrap_err();

        assert!(err.to_string().contains("missing NUL after name"));
    }

    #[test]
    fn tree_parse_rejects_missing_twenty_byte_hash() {
        let raw = b"100644 hello.txt\0too-short";

        let err = Tree::parse(raw).unwrap_err();

        assert!(err.to_string().contains("missing 20-byte object hash"));
    }

    #[test]
    fn tree_parse_handles_multiple_entries() {
        let hash1 = hex_to_hash("ce013625030ba8dba906f756967f9e9ca394464a").unwrap();
        let hash2 = hex_to_hash("f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f").unwrap();

        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 README.md\0");
        raw.extend_from_slice(&hash1);
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&hash2);

        let tree = Tree::parse(&raw).unwrap();

        assert_eq!(tree.entries.len(), 2);

        assert_eq!(tree.entries[0].mode, "100644");
        assert_eq!(tree.entries[0].name, "README.md");
        assert_eq!(tree.entries[0].hash, hash1);

        assert_eq!(tree.entries[1].mode, "100644");
        assert_eq!(tree.entries[1].name, "hello.txt");
        assert_eq!(tree.entries[1].hash, hash2);
    }
}
