use std::{fs, io, todo};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use wg_2024::network::*;
use wg_2024::packet::{Fragment, Packet, PacketType};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, Rgba};
use rand::random;
use crossbeam_channel::{Sender, Receiver};
use petgraph::graph::UnGraph;
use wg_2024::config::Server;
use wg_2024::packet::NackType::Dropped;
use wg_2024::packet::PacketType::MsgFragment;

pub trait MediaServer {//: host
    fn process_image(id:String)->Image{
        let base_path = r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\images\"; // Replace with your actual ID
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
// pub trait ChatServer<M:DroneSend> {
//     fn client_list()-> Vec<NodeId>;
//     fn server_type();
//     fn process_message(client_id:NodeId,message:Message<M>)->(NodeId,Message<M>);
//
// }
// server fntions

pub(crate) struct Image{
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) data: Vec<u8>,
}




















//client fnctions

#[test]
fn main() {
    // let t=convert_image();
    // match revert_image(t) {
    //     Ok(img) => img.save("output_image.png").unwrap(),
    //     Err(e) => eprintln!("Error: {}", e),
    // }
    // let bytes=convert_file();
    // revert_file(bytes).unwrap();
    // {
    //     let mut graph = UnGraph::new_undirected();
    //     let node_index = graph.add_node(0u8);
    //     let node_index2 = graph.add_node(1u8);
    //     graph.add_edge(node_index, node_index2, 1.0);
    //
    //     println!("{},{}", graph.edge_count(),graph.contains_edge(node_index, node_index2));
    // }
    // create_message("i1".to_string());
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
fn revert_file(bytes: Vec<u8>) -> io::Result<()> {
    // Write the bytes directly to a file
    fs::write("output.txt", bytes)?;
    Ok(())
}