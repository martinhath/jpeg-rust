#[allow(unused_variables)]
#[allow(dead_code)]
mod transform;
mod jpeg;
// use std::f32;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::env;

use jpeg::jfif::*;

fn file_to_bytes(path: &Path) -> Vec<u8> {
    if let Ok(file) = File::open(path) {
        return file.bytes()
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect();
    }
    panic!("Coult not open file.")
}

fn main() {
    let mut args = env::args();
    args.next();
    let input_file = args.next().unwrap();
    let output_file = args.next().unwrap();

    let bytes = file_to_bytes(Path::new(&input_file));
    let _ = JFIFImage::parse(bytes, &output_file).unwrap();
}
