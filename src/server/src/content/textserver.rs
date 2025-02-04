use reqwest;
use std::error::Error;

pub trait TextServer {
    fn fetch_file_as_bytes(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // Fetch the file content from the URL
        let response = reqwest::blocking::get(url)?;
        if !response.status().is_success() {
             Err(format!("Failed to fetch file: HTTP {}", response.status()).into())
        }else{
            // Extract the file content as raw bytes
            Ok(response.bytes()?.to_vec())
        }
    }
    fn convert_file(id:String)->Result<Vec<u8>,Box<dyn Error>>{
        // This is the value you want to insert
        let base_path = "https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/files/";
        // Replace it with your actual ID
        let file_name = format!("{}.pdf", id);
        // Create the final path by appending the file name
        let path = format!("{}{}", base_path, file_name);
        if let Ok(bytes)=Self::fetch_file_as_bytes(&path){
            //fetching went well so the file can be sent
            Ok(bytes)
        }else{
            // fetching didn't go well so error in output
            Err(format!("Failed to fetch file: {}", path).into())
        }
    }
}