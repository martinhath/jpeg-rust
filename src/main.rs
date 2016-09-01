#[macro_use]
extern crate itertools;
#[allow(unused_variables)]
#[allow(dead_code)]
mod transform;
mod jpeg;

use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use jpeg::*;

fn file_to_bytes(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    File::open(path).and_then(|mut file| {
        let mut bytes = Vec::new();
        try!(file.read_to_end(&mut bytes));
        Ok(bytes)
    })
}

fn main() {
    let mut args = env::args();
    args.next();
    let input_file = args.next().expect("Must supply an input file");
    let output_file = args.next().expect("Must supply an output file");

    let bytes = file_to_bytes(Path::new(&input_file)).unwrap();
    let image = JPEGImage::parse(bytes).unwrap();
    // Show the image, somehow.

    let mut file = File::create(output_file).unwrap();
    let _ = file.write(format!("P3\n{} {}\n255\n", image.width(), image.height()).as_bytes());
    for &(r, g, b) in image.image_data().unwrap() {
        let s = format!("{} {} {}\n", r, g, b);
        let _ = file.write(s.as_bytes());
    }
}
