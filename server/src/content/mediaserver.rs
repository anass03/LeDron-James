use host::Image;
use image::{save_buffer, ColorType, ImageReader};
use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

/// Error types specific to MediaServer operations.
#[derive(Debug, thiserror::Error)]
pub enum MediaServerError {
    /// Represents a file I/O error.
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    /// Represents an error while decoding or encoding an image.
    #[error("Image Error: {0}")]
    Image(#[from] image::ImageError),
}

/// Trait providing image-related functionality for server components.
pub trait MediaServer {
    /// Loads and decodes an image from the given path.
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

    /// Saves the given image to disk at the specified path.
    fn save_image_to<P: AsRef<Path>>(path: P, image: &Image) -> Result<(), MediaServerError> {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" => {
                // JPEG: remove alpha
                let rgb_data: Vec<u8> = image
                    .data
                    .chunks(4)
                    .flat_map(|chunk| chunk[..3].to_vec())
                    .collect();

                save_buffer(path, &rgb_data, image.width, image.height, ColorType::Rgb8)?;
            }
            _ => {
                // Default: save as RGBA
                save_buffer(
                    path,
                    &image.data,
                    image.width,
                    image.height,
                    ColorType::Rgba8,
                )?;
            }
        }
        Ok(())
    }

    /// Scans public and private directories for image files.
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
