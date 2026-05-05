use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectKind {
    Blob,
    Tree,
}

impl ObjectKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectKind::Blob => "blob",
            ObjectKind::Tree => "tree",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "blob" => Ok(ObjectKind::Blob),
            "tree" => Ok(ObjectKind::Tree),
            _ => bail!("unsupported object type: {s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub kind: ObjectKind,
    pub data: Vec<u8>,
}

impl Object {
    pub fn blob(data: Vec<u8>) -> Self {
        Self {
            kind: ObjectKind::Blob,
            data,
        }
    }

    pub fn tree(data: Vec<u8>) -> Self {
        Self {
            kind: ObjectKind::Tree,
            data,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let header = format!("{} {}\0", self.kind.as_str(), self.data.len());

        let mut out = Vec::with_capacity(header.len() + self.data.len());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(&self.data);

        out
    }

    pub fn deserialize(raw: &[u8]) -> Result<Self> {
        let Some(nul_pos) = raw.iter().position(|&b| b == 0) else {
            bail!("invalid object: missing NUL header terminator");
        };

        let header = std::str::from_utf8(&raw[..nul_pos])
            .context("invalid object header: not valid UTF-8")?;

        let data = raw[nul_pos + 1..].to_vec();

        let mut parts = header.split(' ');

        let Some(kind_str) = parts.next() else {
            bail!("invalid object header: missing type");
        };

        let Some(size_str) = parts.next() else {
            bail!("invalid object header: missing size");
        };

        if parts.next().is_some() {
            bail!("invalid object header: too many fields");
        }

        let kind = ObjectKind::from_str(kind_str)?;
        let size: usize = size_str.parse().context("invalid object size")?;

        if size != data.len() {
            bail!(
                "object size mismatch: header says {}, actual data is {}",
                size,
                data.len()
            );
        }

        Ok(Self { kind, data })
    }

    pub fn hash(&self) -> String {
        let raw = self.serialize();

        let mut hasher = Sha1::new();
        hasher.update(raw);

        hex::encode(hasher.finalize())
    }

    pub fn compress(&self) -> Result<Vec<u8>> {
        let raw = self.serialize();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw)?;
        let compressed = encoder.finish()?;

        Ok(compressed)
    }

    pub fn decompress(compressed: &[u8]) -> Result<Self> {
        let mut decoder = ZlibDecoder::new(compressed);
        let mut raw = Vec::new();

        decoder.read_to_end(&mut raw)?;
        Object::deserialize(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_object_serializes_with_git_header() {
        let object = Object::blob(b"abc".to_vec());

        let serialized = object.serialize();

        assert_eq!(serialized, b"blob 3\0abc");
    }

    #[test]
    fn blob_hash_matches_real_git_for_abc() {
        let object = Object::blob(b"abc".to_vec());

        let hash = object.hash();

        assert_eq!(hash, "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f");
    }

    #[test]
    fn blob_hash_includes_newline_when_present() {
        let object = Object::blob(b"hello world\n".to_vec());

        let hash = object.hash();

        assert_eq!(hash, "3b18e512dba79e4c8300dd08aeb37f8e728b8dad");
    }

    #[test]
    fn object_deserializes_valid_blob() {
        let raw = b"blob 3\0abc";

        let object = Object::deserialize(raw).unwrap();

        assert_eq!(object.kind, ObjectKind::Blob);
        assert_eq!(object.data, b"abc");
    }

    #[test]
    fn deserialize_rejects_missing_nul_separator() {
        let raw = b"blob 3abc";

        let err = Object::deserialize(raw).unwrap_err();

        assert!(err.to_string().contains("missing NUL"));
    }

    #[test]
    fn deserialize_rejects_unknown_object_type() {
        let raw = b"commit 3\0abc";

        let err = Object::deserialize(raw).unwrap_err();

        assert!(err.to_string().contains("unsupported object type"));
    }

    #[test]
    fn deserialize_rejects_size_mismatch() {
        let raw = b"blob 10\0abc";

        let err = Object::deserialize(raw).unwrap_err();

        assert!(err.to_string().contains("object size mismatch"));
    }

    #[test]
    fn compress_and_decompress_round_trip() {
        let original = Object::blob(b"hello from mini-git".to_vec());

        let compressed = original.compress().unwrap();
        let decoded = Object::decompress(&compressed).unwrap();

        assert_eq!(decoded.kind, ObjectKind::Blob);
        assert_eq!(decoded.data, b"hello from mini-git");
        assert_eq!(decoded.hash(), original.hash());
    }

    #[test]
    fn object_kind_string_round_trip() {
        let kind = ObjectKind::from_str("blob").unwrap();

        assert_eq!(kind, ObjectKind::Blob);
        assert_eq!(kind.as_str(), "blob");
    }

    #[test]
    fn tree_object_serializes_with_git_header() {
        let tree_data = b"100644 hello.txt\0abcdefghijklmnopqrst".to_vec();

        let object = Object::tree(tree_data.clone());

        let serialized = object.serialize();

        let mut expected = Vec::new();

        expected.extend_from_slice(format!("tree {}\0", tree_data.len()).as_bytes());
        expected.extend_from_slice(&tree_data);

        assert_eq!(serialized, expected);
    }

    #[test]
    fn object_deserializes_valid_tree() {
        let tree_data = b"100644 hello.txt\0abcdefghijklmnopqrst";

        let mut raw = Vec::new();
        raw.extend_from_slice(format!("tree {}\0", tree_data.len()).as_bytes());
        raw.extend_from_slice(tree_data);

        let object = Object::deserialize(&raw).unwrap();

        assert_eq!(object.kind, ObjectKind::Tree);
        assert_eq!(object.data, tree_data);
    }

    #[test]
    fn tree_object_compress_and_decompress_round_trip() {
        let tree_data = b"100644 hello.txt\0abcdefghijklmnopqrst".to_vec();
        let original = Object::tree(tree_data);

        let compressed = original.compress().unwrap();
        let decoded = Object::decompress(&compressed).unwrap();

        assert_eq!(decoded.kind, ObjectKind::Tree);
        assert_eq!(decoded.data, original.data);
        assert_eq!(decoded.hash(), original.hash());
    }

    #[test]
    fn object_kind_supports_tree() {
        let kind = ObjectKind::from_str("tree").unwrap();

        assert_eq!(kind, ObjectKind::Tree);
        assert_eq!(kind.as_str(), "tree");
    }
}
