use reqwest;
use std::error::Error;
use std::{fs, io};
use std::path::{Path, PathBuf};
use host::FileId;

pub trait TextServer {
    fn read_file_any<P: AsRef<Path>>(folder: P, id: usize, exts: &[&str]) -> io::Result<Vec<u8>> {
        let base: &Path = folder.as_ref();
        for ext in exts {
            let candidate = base.join(format!("i{}.{}", id, ext));
            if candidate.exists() {
                return fs::read(candidate);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Nessun file {} con estensioni {:?} in {:?}", id, exts, base),
        ))
    }
    fn convert_file(id:FileId) -> Result<Vec<u8>, Box<dyn Error>> {

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let files_dir  = manifest_dir.join("src/content/files");
        let id_num = id;
                let bytes = Self::read_file_any(files_dir, id_num, &["txt", "pdf"])?;
        Ok(bytes)
    }
}
