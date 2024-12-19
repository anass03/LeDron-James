use std::fs;
use std::path::PathBuf;

pub trait TextServer {
    fn convert_file(id:String)->Vec<u8>{
        // This is the value you want to insert
        let mut path = PathBuf::new();
        path.push(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\files");
        path.push(format!("{}.txt", id));
        let bytes = fs::read(path).unwrap();
        bytes
    }
}