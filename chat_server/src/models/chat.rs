use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFile {
    pub ws_id: u64,
    pub ext: String,
    pub hash: String,
}

impl ChatFile {
    pub fn new(ws_id: u64, filename: &str, data: &[u8]) -> Self {
        let hash = Sha1::digest(data);
        let (_, ext) = filename.rsplit_once('.').unwrap_or((filename, "txt"));
        Self {
            ws_id,
            ext: ext.to_string(),
            hash: hex::encode(hash),
        }
    }

    pub fn url(&self) -> String {
        format!("/files/{}", self.hash_to_path())
    }

    pub fn path(&self, base_dir: impl AsRef<Path>) -> PathBuf {
        base_dir.as_ref().join(self.hash_to_path())
    }

    pub fn hash_to_path(&self) -> String {
        let (first, remain) = self.hash.split_at(3);
        let (second, third) = remain.split_at(3);
        let ext = &self.ext;
        let ws_id = self.ws_id;
        format!("{ws_id}/{first}/{second}/{third}.{ext}")
    }
}

impl FromStr for ChatFile {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let remain = s
            .strip_prefix("/files/")
            .ok_or(AppError::InvalidInput("file path".to_string()))?;
        let [ws_id, part1, part2, filename] = remain
            .split('/')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| AppError::InvalidInput("file path".to_string()))?;
        let ws_id: u64 = ws_id
            .parse()
            .map_err(|_| AppError::InvalidInput("file path".to_string()))?;
        let [part3, ext] = filename
            .split('.')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| AppError::InvalidInput("file path".to_string()))?;

        let hash = format!("{part1}{part2}{part3}");
        Ok(Self {
            ws_id,
            ext: ext.to_owned(),
            hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use super::*;

    #[test]
    fn chat_file_new_should_work() {
        let file = ChatFile::new(1, "test.txt", b"hello world");
        assert_eq!(file.ws_id, 1);
        assert_eq!(file.ext, "txt");
        assert_eq!(file.hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
        assert_eq!(
            file.hash_to_path(),
            "1/2aa/e6c/35c94fcfb415dbe95f408b9ce91ee846ed.txt"
        );
        assert_eq!(
            file.url(),
            "/files/1/2aa/e6c/35c94fcfb415dbe95f408b9ce91ee846ed.txt"
        );
        assert_eq!(
            file.path("/files"),
            Path::new("/files/1/2aa/e6c/35c94fcfb415dbe95f408b9ce91ee846ed.txt")
        );
    }

    #[test]
    fn parse_valid_url_should_work() {
        let file =
            ChatFile::from_str("/files/1/2aa/e6c/35c94fcfb415dbe95f408b9ce91ee846ed.txt").unwrap();
        assert_eq!(file.ws_id, 1);
        assert_eq!(file.ext, "txt");
        assert_eq!(file.hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn parse_invalid_url_should_work() {
        match ChatFile::from_str("/files/1/2aa/e6c/aa/35c94fcfb415dbe95f408b9ce91ee846ed.txt") {
            Err(AppError::InvalidInput(msg)) => assert_eq!(msg, "file path"),
            _ => panic!("invalid url should return error"),
        };
    }
}
