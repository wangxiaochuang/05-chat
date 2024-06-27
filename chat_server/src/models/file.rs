use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use super::ChatFile;

impl ChatFile {
    pub fn new(filename: &str, data: &[u8]) -> Self {
        let hash = Sha1::digest(data);
        let (_, ext) = filename.rsplit_once('.').unwrap_or((filename, "txt"));
        Self {
            ext: ext.to_string(),
            hash: hex::encode(hash),
        }
    }

    pub fn url(&self, ws_id: u64) -> String {
        format!("/files/{ws_id}/{}", self.hash_to_path())
    }

    pub fn path(&self, base_dir: &Path) -> PathBuf {
        base_dir.join(self.hash_to_path())
    }

    pub fn hash_to_path(&self) -> String {
        let (first, remain) = self.hash.split_at(3);
        let (second, third) = remain.split_at(3);
        let ext = &self.ext;
        format!("{first}/{second}/{third}.{ext}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_file_new_should_work() {
        let file = ChatFile::new("test.txt", b"hello world");
        assert_eq!(file.ext, "txt");
        assert_eq!(file.hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
        assert_eq!(
            file.hash_to_path(),
            "2aa/e6c/35c94fcfb415dbe95f408b9ce91ee846ed.txt"
        )
    }
}
