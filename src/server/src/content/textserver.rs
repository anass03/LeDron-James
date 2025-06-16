use std::fs;
use std::io;
use std::path::PathBuf;
pub trait TextServer {
    fn convert_file_path(path: &PathBuf) -> io::Result<(Vec<u8>, String)> {
        let data = fs::read(path)?;
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        Ok((data, ext))
    }
}