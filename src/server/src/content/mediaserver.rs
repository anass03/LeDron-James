use image::ImageReader;
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
}

