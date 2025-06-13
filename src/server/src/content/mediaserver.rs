use std::error::Error;
use std::{fs, io};
use reqwest;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use image::ImageReader;
use host::{Image, MediaId};
use zstd::stream::encode_all;

pub trait MediaServer {
    /// Cerca "i{id}.{ext}" nella cartella, restituisce (byte, ext) se trovato.
    fn read_file_any_with_ext<P: AsRef<Path>>(folder: P, id: usize, exts: &[&str]) -> io::Result<(Vec<u8>, String)> {
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

    /// Restituisce (Image, estensione) per il file immagine richiesto.
    fn process_image(id: MediaId) -> Result<(Image, String), Box<dyn Error>> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let images_dir = manifest_dir.join("src/content/images");
        let id_num = id;
        let (data_bytes, ext) = Self::read_file_any_with_ext(images_dir, id_num, &["png", "jpeg", "jpg"])?;
        let img = ImageReader::new(Cursor::new(data_bytes)).with_guessed_format()?.decode()?;
        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();
        
        
        let compressed_data = Self::compress_data(&rgba)?;

        Ok((Image {
            width,
            height,
            data: compressed_data,
        }, ext))
    }

    /// Funzione per comprimere i dati
    fn compress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let compressed = encode_all(data, 0)?;
        Ok(compressed)
    }
}
