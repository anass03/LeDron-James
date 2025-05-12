use reqwest;
use std::error::Error;

pub trait TextServer {
    fn fetch_file_as_bytes(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = reqwest::blocking::get(url)?;
        if !response.status().is_success() {
            Err(format!("Failed to fetch file: HTTP {}", response.status()).into())
        } else {
            Ok(response.bytes()?.to_vec())
        }
    }

    fn convert_file(id: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let base_path = "https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/files/";
        let file_name = format!("{}.pdf", id);
        let path = format!("{}{}", base_path, file_name);
        let bytes = Self::fetch_file_as_bytes(&path)?;
        Ok(bytes)
    }
}
