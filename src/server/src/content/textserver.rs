use std::{collections::HashMap, fs, path::{Path, PathBuf}};

/// Errori specifici per il TextServer
#[derive(Debug, thiserror::Error)]
pub enum TextServerError {
    #[error("Errore I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait che fornisce logica condivisa per gestire file di testo
pub trait TextServer {
    /// Legge un file di testo dal disco e restituisce i dati in byte.
    fn convert_file_path<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, TextServerError> {
        Ok(fs::read(path)?)
    }

    /// Scrive un file di testo sul disco
    fn save_text_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), TextServerError> {
        Ok(fs::write(path, data)?)
    }

    /// Scansiona le directory pubblica e privata per file di testo
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