use std::error::Error;
use reqwest;
use std::io::{Cursor};
use image::ImageReader;
use image::DynamicImage;
use host::Image;

pub trait MediaServer {
    fn load_image_from_url(url: &str) -> Result<DynamicImage, Box<dyn Error>> {
        // Fetch the image data from the URL
        let response = reqwest::blocking::get(url).unwrap();
        if !response.status().is_success() {
            Err(format!("Failed to fetch image: HTTP {}", response.status()).into())
        }else{
            // Convert the response body into an in-memory buffer
            let image_data = response.bytes()?;
            let cursor = Cursor::new(image_data);

            // Decode the image from the in-memory buffer
            let img = ImageReader::new(cursor)
                .with_guessed_format()? // Automatically guess the image format
                .decode()?; // Decode into a DynamicImage

            Ok(img)
        }
    }
    fn process_image(id:String)->Result<Image,Box<dyn Error>>{
        let base_path = "https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/images/";
        // Replace it with your actual ID
        let file_name = format!("{}.jpg", id);
        // Create the final path by appending the file name
        let path = format!("{}{}", base_path, file_name);
        if let Ok(img) = Self::load_image_from_url(&path) {
            // Convert the image to raw bytes
            let width = img.width();
            let height = img.height();
            let img = img.to_rgba8();
            let raw_bytes = img.to_vec();
            // Print the length of the byte vector
            println!("Image converted to Vec<u8> with length: {}", raw_bytes.len());
            Ok(Image {
                width,
                height,
                data: raw_bytes,
            })
        }else{
            Err("Failed to convert image".into())
        }
    }
}