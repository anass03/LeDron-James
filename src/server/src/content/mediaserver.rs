use std::error::Error;
use reqwest;
use std::io::Cursor;
use image::ImageReader;
use image::DynamicImage;
use host::Image;

pub trait MediaServer {
    fn load_image_from_url(url: &str) -> Result<DynamicImage, Box<dyn Error>> {
        let response = reqwest::blocking::get(url)?;
        if !response.status().is_success() {
            Err(format!("Failed to fetch image: HTTP {}", response.status()).into())
        } else {
            let image_data = response.bytes()?;
            let cursor = Cursor::new(image_data);
            let img = ImageReader::new(cursor)
                .with_guessed_format()?
                .decode()?;
            Ok(img)
        }
    }

    fn process_image(id: String) -> Result<Image, Box<dyn Error>> {
        let base_path = "https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/images/";
        let file_name = format!("{}.jpg", id);
        let path = format!("{}{}", base_path, file_name);
        let img = Self::load_image_from_url(&path)?;
        let width = img.width();
        let height = img.height();
        let img = img.to_rgba8();
        let raw_bytes = img.to_vec();
        Ok(Image {
            width,
            height,
            data: raw_bytes,
        })
    }
}