use std::{collections::HashMap, fs, io::Cursor, path::{Path, PathBuf}};
use image::{save_buffer, ColorType, ImageReader};
use host::Image;

/// Errori specifici per MediaServer, al posto di Box<dyn Error>
#[derive(Debug, thiserror::Error)]
pub enum MediaServerError {
    #[error("Errore I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Errore immagine: {0}")]
    Image(#[from] image::ImageError),
}

pub trait MediaServer {
    /// Carica e decodifica un'immagine dal path dato
    fn process_image_path<P: AsRef<Path>>(path: P) -> Result<Image, MediaServerError> {
        let data_bytes = fs::read(&path)?;
        let img = ImageReader::new(Cursor::new(&data_bytes))
            .with_guessed_format()?
            .decode()?;

        Ok(Image {
            width: img.width(),
            height: img.height(),
            data: img.to_rgba8().to_vec(),
        })
    }

    /// Salva l'immagine su disco nel path specificato
    fn save_image_to<P: AsRef<Path>>(path: P, image: &Image) -> Result<(), MediaServerError> {
        save_buffer(
            path,
            &image.data,
            image.width,
            image.height,
            ColorType::Rgba8,
        )?;
        Ok(())
    }

    /// Scansiona le directory pubblica e privata per file immagine
    fn scan_media_files(&self, private_images_dir: &Path) -> HashMap<String, PathBuf> {
        let public_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/content/images");
        [&public_dir, private_images_dir]
            .iter()
            .flat_map(|dir| fs::read_dir(dir).into_iter().flatten())
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    Some((filename.to_string(), path.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}
