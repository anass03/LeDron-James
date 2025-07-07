use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// Error types specific to the `TextServer` operations.
#[derive(Debug, thiserror::Error)]
pub enum TextServerError {
    /// I/O error wrapper.
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait providing logic to manage text files for servers.
pub trait TextServer {
    /// Reads a text file from the disk and returns its content as bytes.
    fn convert_file_path<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, TextServerError> {
        Ok(fs::read(path)?)
    }

    /// Writes a text file to disk with the provided byte content.
    fn save_text_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), TextServerError> {
        Ok(fs::write(path, data)?)
    }

    /// Scans both public and private directories for available text files and returns a mapping of file names to paths.
    fn scan_text_files(&self, private_files_dir: &Path) -> HashMap<String, PathBuf> {
        let public_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/content/files");

        [&public_dir, private_files_dir]
            .iter()
            .flat_map(|dir| fs::read_dir(dir).into_iter().flatten())
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| (name.to_string(), path.clone()))
            })
            .collect()
    }
}
