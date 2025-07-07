use std::error::Error;
use std::io::Cursor;
use zstd::{decode_all, encode_all};

pub fn compress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let compressed = encode_all(data, 3)?;
    Ok(compressed)
}
pub fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let decompressed = decode_all(Cursor::new(compressed))?;
    Ok(decompressed)
}
