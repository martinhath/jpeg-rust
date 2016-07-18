#[allow(unused_variables)]
#[allow(dead_code)]
mod jpeg;
use std::f32::consts::PI;
use std::f32;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use jpeg::jfif::*;

#[allow(non_upper_case_globals)]
const Pi: f32 = PI as f32;

struct SquareMatrix {
    dimension: usize,
    values: Vec<f32>,
}

impl SquareMatrix {
    fn print(&self) {
        let d = self.dimension;
        for i in 0..d {
            for j in 0..d {
                let a = d * i + j;
                print!("{:7.2}  ", self.values[a]);
            }
            print!("\n");
        }
        println!("");
    }
}

fn discrete_cosine_transform(mat: &SquareMatrix) -> SquareMatrix {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = mat.dimension;
    let mut vec = Vec::with_capacity(mat.values.len());

    for v in 0..d {
        for u in 0..d {
            let vf = v as f32;
            let uf = u as f32;

            let mut sum = 0f32;
            for y in 0..d {
                for x in 0..d {
                    let xy_index = y * d + x;
                    let gxy = mat.values[xy_index] as f32;

                    let yf = y as f32;
                    let xf = x as f32;

                    let prod = gxy * ((2f32 * xf + 1f32) * uf * Pi / 16f32).cos() *
                               ((2f32 * yf + 1f32) * vf * Pi / 16f32).cos();
                    sum += prod;
                }
            }
            let g_uv = alpha(u) * alpha(v) * sum / 4f32;
            vec.push(g_uv);
        }
    }

    SquareMatrix {
        dimension: d,
        values: vec,
    }
}

fn discrete_cosine_transform_inverse(mat: &SquareMatrix) -> SquareMatrix {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = mat.dimension;
    let mut vec = Vec::with_capacity(d * d);

    for y in 0..d {
        for x in 0..d {
            let yf = y as f32;
            let xf = x as f32;
            let mut sum = 0f32;
            for v in 0..d {
                for u in 0..d {
                    let uf = u as f32;
                    let vf = v as f32;

                    let f_uv = mat.values[v * d + u];
                    sum += alpha(u) * alpha(v) * f_uv *
                           ((2f32 * xf + 1f32) * uf * Pi / 16f32).cos() *
                           ((2f32 * yf + 1f32) * vf * Pi / 16f32).cos();
                }
            }
            vec.push(sum / 4f32);
        }
    }


    SquareMatrix {
        dimension: d,
        values: vec,
    }
}

// TODO: do this not retarded
fn inner_div_round(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
    let d = a.dimension;
    if d != b.dimension {
        panic!("Matrix dimensions must be the same");
    }
    let mut vec = Vec::with_capacity(d * d);
    for j in 0..d {
        for i in 0..d {
            let index = j * d + i;
            vec.push((a.values[index] / b.values[index]).round());
        }
    }
    SquareMatrix {
        dimension: d,
        values: vec,
    }
}

// TODO: do this not retarded
fn inner_mul(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
    let d = a.dimension;
    if d != b.dimension {
        panic!("Matrix dimensions must be the same");
    }
    let mut vec = Vec::with_capacity(d * d);
    for j in 0..d {
        for i in 0..d {
            let index = j * d + i;
            vec.push(a.values[index] * b.values[index]);
        }
    }
    SquareMatrix {
        dimension: d,
        values: vec,
    }
}

#[allow(dead_code)]
fn decode(mat: SquareMatrix) -> SquareMatrix {
    println!("decode()");
    mat.print();
    let dequantized = inner_mul(&mat, &quantization_matrix());
    let mut spatial = discrete_cosine_transform_inverse(&dequantized);
    for n in spatial.values.iter_mut() {
        *n = n.round();
    }
    for n in spatial.values.iter_mut() {
        *n += 128f32;
    }
    spatial.print();
    spatial
}

#[allow(dead_code)]
fn encode(mat: SquareMatrix) -> SquareMatrix {
    println!("encode()");
    mat.print();
    let mut mat = mat;
    for n in mat.values.iter_mut() {
        *n -= 128f32;
    }
    let transformed = discrete_cosine_transform(&mat);
    let quantized = inner_div_round(&transformed, &quantization_matrix());
    quantized.print();
    quantized
}



#[allow(dead_code)]
fn quantization_matrix() -> SquareMatrix {
    SquareMatrix {
        dimension: 8,
        values: vec![16f32, 11f32, 10f32, 16f32, 24f32, 40f32, 51f32, 61f32, 12f32, 12f32, 14f32,
                     19f32, 26f32, 58f32, 60f32, 55f32, 14f32, 13f32, 16f32, 24f32, 40f32, 57f32,
                     69f32, 56f32, 14f32, 17f32, 22f32, 29f32, 51f32, 87f32, 80f32, 62f32, 18f32,
                     22f32, 37f32, 56f32, 68f32, 109f32, 103f32, 77f32, 24f32, 35f32, 55f32,
                     64f32, 81f32, 104f32, 113f32, 92f32, 49f32, 64f32, 78f32, 87f32, 103f32,
                     121f32, 120f32, 101f32, 72f32, 92f32, 95f32, 98f32, 112f32, 100f32, 103f32,
                     99f32],
    }
}

#[allow(dead_code)]
fn encoded_matrix() -> SquareMatrix {
    SquareMatrix {
        dimension: 8,
        values: vec![-26f32, -3f32, -6f32, 2f32, 2f32, -1f32, 0f32, 0f32, 0f32, -2f32, -4f32,
                     1f32, 1f32, 0f32, 0f32, 0f32, -3f32, 1f32, 5f32, -1f32, -1f32, 0f32, 0f32,
                     0f32, -3f32, 1f32, 2f32, -1f32, 0f32, 0f32, 0f32, 0f32, 1f32, 0f32, 0f32,
                     0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
                     0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
                     0f32, 0f32, 0f32],
    }
}

#[allow(dead_code)]
fn sample_matrix() -> SquareMatrix {
    SquareMatrix {
        dimension: 8,
        values: vec![52f32, 55f32, 61f32, 66f32, 70f32, 61f32, 64f32, 73f32, 63f32, 59f32, 55f32,
                     90f32, 109f32, 85f32, 69f32, 72f32, 62f32, 59f32, 68f32, 113f32, 144f32,
                     104f32, 66f32, 73f32, 63f32, 58f32, 71f32, 122f32, 154f32, 106f32, 70f32,
                     69f32, 67f32, 61f32, 68f32, 104f32, 126f32, 88f32, 68f32, 70f32, 79f32,
                     65f32, 60f32, 70f32, 77f32, 68f32, 58f32, 75f32, 85f32, 71f32, 64f32, 59f32,
                     55f32, 61f32, 65f32, 83f32, 87f32, 79f32, 69f32, 68f32, 65f32, 76f32, 78f32,
                     94f32],
    }
}

#[allow(dead_code)]
fn error_matrix(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
    let d = a.dimension;
    let mut vec = Vec::with_capacity(d * d);
    for y in 0..d {
        for x in 0..d {
            let index = y * d + x;
            vec.push(a.values[index] - b.values[index]);
        }
    }
    SquareMatrix {
        dimension: d,
        values: vec,
    }
}

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
    // let mut matrix = sample_matrix();
    // let encoded = encode(matrix);
    // let decoded = decode(encoded);

    // let error = error_matrix(&sample_matrix(), &decoded);
    // error.print();

    let bytes = file_to_bytes(Path::new("./lena-bw.jpeg"));

    let image = JFIFImage::parse(bytes).unwrap();

    let first = image.get_nth_square(0);
    let mat = SquareMatrix {
        dimension: 8,
        values: first.to_owned().iter().map(|b| *b as f32).collect(),
    };
    let _ = decode(mat);
}
