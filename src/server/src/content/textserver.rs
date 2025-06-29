use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// TextServer trait provides shared logic for serving text files to clients.
/// It allows reading files from disk and preparing them for transmission.
pub trait TextServer {
    /// Legge un file di testo dal disco e restituisce i dati in byte.
    ///
    /// # Argomenti
    /// * `path` - Path completo del file.
    ///
    /// # Ritorna
    /// * `Ok(data)` - Contenuto del file come `Vec<u8>`
    /// * `Err` - In caso di errore di lettura.
    fn convert_file_path(path: &PathBuf) -> std::io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn save_text_file(path: &PathBuf, data: &[u8]) -> std::io::Result<()> {
        fs::write(path, data) 
    }
    fn scan_text_files(&self, private_files_dir: &PathBuf) -> HashMap<String, PathBuf> { 
        let mut map = HashMap::new();

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let public_dir = manifest_dir.join("src/content/files");

        for dir in [&public_dir, private_files_dir] {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                            map.entry(filename.to_string()).or_insert(path);
                        }
                    }
                }
            }
        }
        map
    }
}

