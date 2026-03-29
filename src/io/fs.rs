use std::{fs, path::PathBuf, time::SystemTime};

use anyhow::Result;

#[derive(Clone, Debug)]
pub struct FileSystemDocumentSource {
    path: PathBuf,
}

impl FileSystemDocumentSource {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn read_to_string(&self) -> Result<String> {
        Ok(fs::read_to_string(&self.path)?)
    }

    pub fn modified_at(&self) -> Result<SystemTime> {
        Ok(fs::metadata(&self.path)?.modified()?)
    }

    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
