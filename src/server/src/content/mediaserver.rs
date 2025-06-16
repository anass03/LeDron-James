use image::ImageReader;
use std::fs;
use std::io::{Cursor};
use std::path::PathBuf;
use host::Image;

pub trait MediaServer {
    fn process_image_path(path: &PathBuf) -> Result<(Image, String), Box<dyn std::error::Error>> {
        let data_bytes = fs::read(path).map_err(|e| {
            eprintln!("Error reading image: {}", e);
            e
        })?;
        let img = ImageReader::new(Cursor::new(&data_bytes))
            .with_guessed_format()?
            .decode()?;

        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Ok((Image { width, height, data: rgba.to_vec() }, ext))
    }
}
