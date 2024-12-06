use wg_2024::network::*;
use wg_2024::packet::ServerType;
use crate::message::*;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, Rgba};


pub trait MediaServer {//: host
    fn processing_image();
    fn processing_text();
    fn server_type() -> ServerType;


}
pub trait ChatServer<M:DroneSend> {
    fn client_list()-> Vec<NodeId>;
    fn server_type();
    fn process_message(client_id:NodeId,message:Message<M>)->(NodeId,Message<M>);

}
struct Image{
    width: u32,
    height: u32,
    data: Vec<u8>,
}
fn convert_image()->Image{
    let path = r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\images\i1.jpg"; // Replace with your image path
    let img: DynamicImage = ImageReader::open(path).unwrap().decode().unwrap();
    // Convert the image to raw bytes
    let width = img.width();
    let height = img.height();
    let img = img.to_rgba8();
    let raw_bytes: Vec<u8> = img.to_vec();

    // Print the length of the byte vector
    println!("Image converted to Vec<u8> with length: {}", raw_bytes.len());
    Image{
        width,
        height,
        data: raw_bytes,
    }
}
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
#[test]
fn main() {
    let t=convert_image();
    match revert_image(t) {
        Ok(img) => img.save("output_image.png").unwrap(),
        Err(e) => eprintln!("Error: {}", e),
    }
}