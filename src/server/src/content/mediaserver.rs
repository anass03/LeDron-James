use std::collections::HashMap;
use image::{save_buffer, ColorType, ImageReader};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use host::Image;

/// MediaServer trait provides shared logic for media (image) processing.
/// It allows servers to load images from disk and convert them into serializable
/// structures that can be transmitted to clients.
pub trait MediaServer {
    /// Legge e decodifica un'immagine da disco, restituendo una struct `Image` serializzabile.
    ///
    /// # Argomenti
    /// * `path` - Path dell'immagine.
    ///
    /// # Ritorna
    /// * `Ok(Image)` se corretta.
    /// * `Err` in caso di errore I/O o decodifica.
    fn process_image_path(path: &PathBuf) -> Result<Image, Box<dyn std::error::Error>> {
        let data_bytes = fs::read(path)?;
        let img = ImageReader::new(Cursor::new(&data_bytes))
            .with_guessed_format()?
            .decode()?;
        
        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();
        
        Ok(Image {
            width,
            height,
            data: rgba.to_vec(),
        })
    }
    fn save_image_to(path: &PathBuf, image: &Image) -> Result<(), Box<dyn std::error::Error>> {
        save_buffer(
            path,
            &image.data,
            image.width,
            image.height,
            ColorType::Rgba8,
        )?;
        Ok(())
    }
    fn scan_media_files(&self, private_images_dir: &PathBuf) -> HashMap<String, PathBuf> {
        let mut map = HashMap::new();

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let public_dir = manifest_dir.join("src/content/images");

        for dir in [&public_dir, private_images_dir] {
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

