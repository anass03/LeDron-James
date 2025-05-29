use std::error::Error;
use std::{fs, io};
use reqwest;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use image::ImageReader;
use image::DynamicImage;
use host::{Image, MediaId};

pub trait MediaServer {
    fn read_file_any<P: AsRef<Path>>(folder: P, id: usize, exts: &[&str]) -> io::Result<Vec<u8>> {
        let base: &Path = folder.as_ref();
        for ext in exts {
            let candidate = base.join(format!("i{}.{}", id, ext));
            if candidate.exists() {
                return fs::read(candidate);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Nessun file {} con estensioni {:?} in {:?}", id, exts, base),
        ))
    }
    
        /// Carica un’immagine PNG/JPEG locale con nome numerico 
        fn process_image(id: MediaId) -> Result<Image, Box<dyn Error>> {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

            let images_dir  = manifest_dir.join("src/content/images");
            let id_num = id;
                let data = Self::read_file_any(images_dir, id_num, &["png", "jpeg", "jpg"])?;
            // Decodifica da bytes a DynamicImage
            let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()?
            .decode()?;
            let width = img.width();
            let height = img.height();
            let rgba = img.to_rgba8();
            Ok(Image {
            width,
            height,
            data: rgba.to_vec(),
            })
        }
}