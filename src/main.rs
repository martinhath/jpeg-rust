use std::f32;
use std::f32::consts::PI;

const Pi: f32 = PI as f32;

struct SquareMatrix {
    dimention: usize,
    values: Vec<f32>,
}

impl SquareMatrix {
    fn print(&self) {
        let d = self.dimention;
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

// NOTE: encoding
fn sample_matrix() -> SquareMatrix {
    let mut vec = vec![52f32, 55f32, 61f32, 66f32, 70f32, 61f32, 64f32, 73f32, 63f32, 59f32,
                       55f32, 90f32, 109f32, 85f32, 69f32, 72f32, 62f32, 59f32, 68f32, 113f32,
                       144f32, 104f32, 66f32, 73f32, 63f32, 58f32, 71f32, 122f32, 154f32, 106f32,
                       70f32, 69f32, 67f32, 61f32, 68f32, 104f32, 126f32, 88f32, 68f32, 70f32,
                       79f32, 65f32, 60f32, 70f32, 77f32, 68f32, 58f32, 75f32, 85f32, 71f32,
                       64f32, 59f32, 55f32, 61f32, 65f32, 83f32, 87f32, 79f32, 69f32, 68f32,
                       65f32, 76f32, 78f32, 94f32];

    SquareMatrix {
        dimention: 8,
        values: vec,
    }
}

// NOTE: encoding
fn discrete_cosine_transform(mat: &SquareMatrix) -> SquareMatrix {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = mat.dimention;
    let mut vec = Vec::with_capacity(mat.values.len());

    for v in 0..d {
        for u in 0..d {
            let index = v * d + u;
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
            let Guv = alpha(u) * alpha(v) * sum / 4f32;
            vec.push(Guv);
        }
    }

    SquareMatrix {
        dimention: d,
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
    let d = mat.dimention;
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

                    let Fuv = mat.values[v * d + u];
                    sum += alpha(u) * alpha(v) * Fuv *
                           ((2f32 * xf + 1f32) * uf * Pi / 16f32).cos() *
                           ((2f32 * yf + 1f32) * vf * Pi / 16f32).cos();
                }
            }
            vec.push(sum / 4f32);
        }
    }


    SquareMatrix {
        dimention: d,
        values: vec,
    }
}

fn quantization_matrix() -> SquareMatrix {
    SquareMatrix {
        dimention: 8,
        values: vec![16f32, 11f32, 10f32, 16f32, 24f32, 40f32, 51f32, 61f32, 12f32, 12f32, 14f32,
                     19f32, 26f32, 58f32, 60f32, 55f32, 14f32, 13f32, 16f32, 24f32, 40f32, 57f32,
                     69f32, 56f32, 14f32, 17f32, 22f32, 29f32, 51f32, 87f32, 80f32, 62f32, 18f32,
                     22f32, 37f32, 56f32, 68f32, 109f32, 103f32, 77f32, 24f32, 35f32, 55f32,
                     64f32, 81f32, 104f32, 113f32, 92f32, 49f32, 64f32, 78f32, 87f32, 103f32,
                     121f32, 120f32, 101f32, 72f32, 92f32, 95f32, 98f32, 112f32, 100f32, 103f32,
                     99f32],
    }
}

fn inner_div(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
    let d = a.dimention;
    if d != b.dimention {
        panic!("Matrix dimentions must be the same");
    }
    let mut vec = Vec::with_capacity(d * d);
    for j in 0..d {
        for i in 0..d {
            let index = j * d + i;
            vec.push(a.values[index] / b.values[index]);
        }
    }
    SquareMatrix {
        dimention: d,
        values: vec,
    }
}

fn inner_mul(a: &SquareMatrix, b: &SquareMatrix) -> SquareMatrix {
    let d = a.dimention;
    if d != b.dimention {
        panic!("Matrix dimentions must be the same");
    }
    let mut vec = Vec::with_capacity(d * d);
    for j in 0..d {
        for i in 0..d {
            let index = j * d + i;
            vec.push(a.values[index] * b.values[index]);
        }
    }
    SquareMatrix {
        dimention: d,
        values: vec,
    }
}

fn decode(mat: SquareMatrix) -> SquareMatrix {
    let dequantized = inner_mul(&mat, &quantization_matrix());
    println!("dequantized");
    dequantized.print();
    // name?
    let mut spatial = discrete_cosine_transform_inverse(&dequantized);
    println!("after transform and round");
    for n in spatial.values.iter_mut() {
        *n = n.round();
    }
    spatial.print();

    for n in spatial.values.iter_mut() {
        *n += 128f32;
    }
    println!("Add back 128");
    spatial.print();
    spatial
}

fn encoded_matrix() -> SquareMatrix {
    SquareMatrix {
        dimention: 8,
        values: vec![-26f32, -3f32, -6f32, 2f32, 2f32, -1f32, 0f32, 0f32, 0f32, -2f32, -4f32,
                     1f32, 1f32, 0f32, 0f32, 0f32, -3f32, 1f32, 5f32, -1f32, -1f32, 0f32, 0f32,
                     0f32, -3f32, 1f32, 2f32, -1f32, 0f32, 0f32, 0f32, 0f32, 1f32, 0f32, 0f32,
                     0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
                     0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32,
                     0f32, 0f32, 0f32],
    }
}

fn main() {
    let mut matrix = encoded_matrix();// sample_matrix();
    matrix.print();

    decode(matrix);

    //     for a in matrix.values.iter_mut() {
    //         *a -= 128f32;
    //     }
    //     matrix.print();
    //     println!("");
    //
    //     let mut transformed = discrete_cosine_transform(&matrix);
    //
    // transformed.print();
    println!("");
}
