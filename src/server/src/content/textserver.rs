use reqwest;
use std::io::{self, BufRead};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

pub trait TextServer {
    fn fetch_file_as_bytes(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // Fetch the file content from the URL
        let response = reqwest::blocking::get(url)?;
        if !response.status().is_success() {
            return Err(format!("Failed to fetch file: HTTP {}", response.status()).into());
        }
        // Extract the file content as raw bytes
        Ok(response.bytes()?.to_vec())
    }
    fn convert_file(id:String)->Vec<u8>{
        // This is the value you want to insert
        let mut path = String::new();
        path.push_str(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\files");
        path.push_str(format!("{}.txt", id).as_str());
        let bytes = Self::fetch_file_as_bytes(&path).unwrap();
        bytes
    }
}