#[allow(unused_variables)]
#[allow(dead_code)]
mod transform;
mod jpeg;
// use std::f32;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use jpeg::jfif::*;

// struct SquareMatrix {
//     dimension: usize,
//     values: Vec<f32>,
// }
//
// impl SquareMatrix {
//     fn print(&self) {
//         let d = self.dimension;
//         for i in 0..d {
//             for j in 0..d {
//                 let a = d * i + j;
//                 print!("{:7.2}  ", self.values[a]);
//             }
//             print!("\n");
//         }
//         println!("");
//     }
// }
//
// // TODO: do this not retarded
// fn inner_div_round(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
//     let d = a.dimension;
//     if d != b.dimension {
//         panic!("Matrix dimensions must be the same");
//     }
//     let mut vec = Vec::with_capacity(d * d);
//     for j in 0..d {
//         for i in 0..d {
//             let index = j * d + i;
//             vec.push((a.values[index] / b.values[index]).round());
//         }
//     }
//     SquareMatrix {
//         dimension: d,
//         values: vec,
//     }
// }
//
// // TODO: do this not retarded
// fn inner_mul(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
//     let d = a.dimension;
//     if d != b.dimension {
//         panic!("Matrix dimensions must be the same");
//     }
//     let mut vec = Vec::with_capacity(d * d);
//     for j in 0..d {
//         for i in 0..d {
//             let index = j * d + i;
//             vec.push(a.values[index] * b.values[index]);
//         }
//     }
//     SquareMatrix {
//         dimension: d,
//         values: vec,
//     }
// }
//
// #[allow(dead_code)]
// fn decode(mat: SquareMatrix) -> SquareMatrix {
//     println!("decode()");
//     mat.print();
//     let dequantized = inner_mul(&mat, &quantization_matrix());
//     let mut spatial = transform::discrete_cosine_transform_inverse(&dequantized);
//     for n in spatial.values.iter_mut() {
//         *n = n.round();
//     }
//     for n in spatial.values.iter_mut() {
//         *n += 128f32;
//     }
//     spatial.print();
//     spatial
// }
//
// #[allow(dead_code)]
// fn encode(mat: SquareMatrix) -> SquareMatrix {
//     println!("encode()");
//     mat.print();
//     let mut mat = mat;
//     for n in mat.values.iter_mut() {
//         *n -= 128f32;
//     }
//     let transformed = transform::discrete_cosine_transform(&mat);
//     let quantized = inner_div_round(&transformed, &quantization_matrix());
//     quantized.print();
//     quantized
// }
//
//
//
// #[allow(dead_code)]
// fn quantization_matrix() -> SquareMatrix {
//     SquareMatrix {
//         dimension: 8,
//         values: vec![16f32, 11f32, 10f32, 16f32, 24f32, 40f32, 51f32, 61f32, 12f32, 12f32, 14f32,
//                      19f32, 26f32, 58f32, 60f32, 55f32, 14f32, 13f32, 16f32, 24f32, 40f32, 57f32,
//                      69f32, 56f32, 14f32, 17f32, 22f32, 29f32, 51f32, 87f32, 80f32, 62f32, 18f32,
//                      22f32, 37f32, 56f32, 68f32, 109f32, 103f32, 77f32, 24f32, 35f32, 55f32,
//                      64f32, 81f32, 104f32, 113f32, 92f32, 49f32, 64f32, 78f32, 87f32, 103f32,
//                      121f32, 120f32, 101f32, 72f32, 92f32, 95f32, 98f32, 112f32, 100f32, 103f32,
//                      99f32],
//     }
// }
//
// #[allow(dead_code)]
// fn encoded_matrix() -> SquareMatrix {
//     SquareMatrix {
//         dimension: 8,
//         values: vec![-26f32, -3f32, -6f32, 2f32, 2f32, -1f32, 0f32, 0f32, 0f32, -2f32, -4f32,
//                      1f32, 1f32, 0f32, 0f32, 0f32, -3f32, 1f32, 5f32, -1f32, -1f32, 0f32, 0f32,
//                      0f32, -3f32, 1f32, 2f32, -1f32, 0f32, 0f32, 0f32, 0f32, 1f32, 0f32, 0f32,
//                      0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
//                      0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
//                      0f32, 0f32, 0f32],
//     }
// }
//
// #[allow(dead_code)]
// fn sample_matrix() -> SquareMatrix {
//     SquareMatrix {
//         dimension: 8,
//         values: vec![52f32, 55f32, 61f32, 66f32, 70f32, 61f32, 64f32, 73f32, 63f32, 59f32, 55f32,
//                      90f32, 109f32, 85f32, 69f32, 72f32, 62f32, 59f32, 68f32, 113f32, 144f32,
//                      104f32, 66f32, 73f32, 63f32, 58f32, 71f32, 122f32, 154f32, 106f32, 70f32,
//                      69f32, 67f32, 61f32, 68f32, 104f32, 126f32, 88f32, 68f32, 70f32, 79f32,
//                      65f32, 60f32, 70f32, 77f32, 68f32, 58f32, 75f32, 85f32, 71f32, 64f32, 59f32,
//                      55f32, 61f32, 65f32, 83f32, 87f32, 79f32, 69f32, 68f32, 65f32, 76f32, 78f32,
//                      94f32],
//     }
// }
//
// #[allow(dead_code)]
// fn error_matrix(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
//     let d = a.dimension;
//     let mut vec = Vec::with_capacity(d * d);
//     for y in 0..d {
//         for x in 0..d {
//             let index = y * d + x;
//             vec.push(a.values[index] - b.values[index]);
//         }
//     }
//     SquareMatrix {
//         dimension: d,
//         values: vec,
//     }
// }

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
    // let bytes = file_to_bytes(Path::new("./lena-bw.jpeg"));
    let bytes = file_to_bytes(Path::new("./huff_simple0.jpg"));

    let _ = JFIFImage::parse(bytes).unwrap();
}
