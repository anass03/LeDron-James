use image::ImageReader;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use host::Image;

/// MediaServer trait provides shared logic for media (image) processing.
/// It allows servers to load images from disk and convert them into serializable
/// structures that can be transmitted to clients.
pub trait MediaServer {
    /// Reads an image file from disk, decodes it, and converts it into an internal Image struct.
    ///
    /// # Arguments
    /// * `path` - The path to the image file on disk.
    ///
    /// # Returns
    /// * `Ok((Image, extension))` if successful:
    ///     - Image: containing width, height, and raw RGBA bytes.
    ///     - extension: file extension (e.g., "png", "jpg") as String.
    /// * `Err` - In case of any I/O or decoding failure.
    fn process_image_path(path: &PathBuf) -> Result<(Image, String), Box<dyn std::error::Error>> {
        // Read the raw image file contents
        let data_bytes = fs::read(path).map_err(|e| {
            eprintln!("Error reading image: {}", e);
            e
        })?;

        // Use ImageReader to decode and guess format automatically
        let img = ImageReader::new(Cursor::new(&data_bytes))
            .with_guessed_format()?
            .decode()?;

        // Extract image metadata and convert image to raw RGBA bytes
        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();

        // Extract file extension (if available)
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Ok((Image { width, height, data: rgba.to_vec() }, ext))
    }
}
