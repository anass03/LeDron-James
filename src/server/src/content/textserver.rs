use std::error::Error;
use std::{fs, io};
use std::path::{Path, PathBuf};
use host::FileId;

pub trait TextServer {
    /// Legge "i{id}.{ext}" per ognuna delle exts e restituisce (byte, ext) se trovato.
    fn read_file_any_with_ext<P: AsRef<Path>>(folder: P, id: usize, exts: &[&str])
        -> io::Result<(Vec<u8>, String)> {
        let base: &Path = folder.as_ref();
        for ext in exts {
            let candidate = base.join(format!("i{}.{}", id, ext));
            if candidate.exists() {
                let data = fs::read(&candidate)?;
                return Ok((data, ext.to_string()));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Nessun file {} con estensioni {:?} in {:?}", id, exts, base),
        ))
    }

    /// Restituisce (contenuto, estensione) del file richiesto come Text.
    fn convert_file(id: FileId) -> Result<(Vec<u8>, String), Box<dyn Error>> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files_dir  = manifest_dir.join("src/content/files");
        let id_num = id;
        let (bytes, ext) = Self::read_file_any_with_ext(files_dir, id_num, &["txt", "pdf"])?;
        Ok((bytes, ext))
    }
}
