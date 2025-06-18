use std::fs;
use std::io;
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
    fn convert_file_path(path: &PathBuf) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
}

