use std::{fs, io};
use std::path::PathBuf;
use image::ImageReader;
use image::{DynamicImage, ImageBuffer, Rgba};
pub trait MediaServer {//: host
    fn process_image(id:String)->Image{
        let base_path = r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\images\"; // Replace it with your actual ID
        let file_name = format!("{}.jpg", id);
        // Create the final path by appending the file name
        let mut path = PathBuf::from(base_path);
        path.push(file_name);
        let img: DynamicImage = ImageReader::open(path).unwrap().decode().unwrap();
        // Convert the image to raw bytes
        let width = img.width();
        let height = img.height();
        let img = img.to_rgba8();
        let raw_bytes = img.to_vec();

        // Print the length of the byte vector
        println!("Image converted to Vec<u8> with length: {}", raw_bytes.len());
        Image{
            width,
            height,
            data: raw_bytes,
        }
    }
}
pub struct Image{
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
//client functions
fn revert_image(image:Image) -> Result<DynamicImage, String> {
    if image.data.len() != (image.width * image.height * 4) as usize {
        return Err(format!(
            "Byte vector size mismatch: expected {}, got {}",
            image.width * image.height * 4,
            image.data.len()
        ));
    }
    // Try creating the image buffer
    let img_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(image.width, image.height, image.data)
        .ok_or("Failed to create ImageBuffer from raw bytes")?;

    Ok(DynamicImage::ImageRgba8(img_buffer))
}
fn revert_file(bytes: Vec<u8>) -> io::Result<()> {
    // Write the bytes directly to a file
    fs::write("output.txt", bytes)?;
    Ok(())
}