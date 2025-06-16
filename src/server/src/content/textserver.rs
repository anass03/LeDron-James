use std::fs;
use std::io;
use std::path::PathBuf;

/// TextServer trait provides shared logic for serving text files to clients.
/// It allows reading files from disk and preparing them for transmission.
pub trait TextServer {
    /// Reads a text file from disk and extracts both its content and extension.
    ///
    /// # Arguments
    /// * `path` - The file path to load.
    ///
    /// # Returns
    /// * `Ok((data, extension))` if successful:
    ///     - `data`: raw byte content of the file.
    ///     - `extension`: file extension as String (e.g. "txt").
    /// * `Err` if the file cannot be read.
    fn convert_file_path(path: &PathBuf) -> io::Result<(Vec<u8>, String)> {
        // Read full file contents as raw bytes
        let data = fs::read(path)?;

        // Extract file extension (if present)
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Ok((data, ext))
    }
}
