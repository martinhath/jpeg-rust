
struct SquareMatrix<N> {
    dimention: usize,
    values: Vec<N>,
}

impl<F> SquareMatrix<F>
    where F: std::fmt::Display
{
    fn print(&self) {
        let d = self.dimention;
        for i in 0..d {
            for j in 0..d {
                let a = d * i + j;
                print!("{:7.2}  ", self.values[a]);
            }
            print!("\n");
        }
    }
}

fn sample_matrix() -> SquareMatrix<i32> {
    let mut vec = vec![52, 55, 61, 66, 70, 61, 64, 73, 63, 59, 55, 90, 109, 85, 69, 72, 62, 59,
                       68, 113, 144, 104, 66, 73, 63, 58, 71, 122, 154, 106, 70, 69, 67, 61, 68,
                       104, 126, 88, 68, 70, 79, 65, 60, 70, 77, 68, 58, 75, 85, 71, 64, 59, 55,
                       61, 65, 83, 87, 79, 69, 68, 65, 76, 78, 94];

    SquareMatrix {
        dimention: 8,
        values: vec,
    }
}

use std::f32;
use std::f32::consts::PI;

const Pi: f32 = PI as f32;

fn discrete_cosinus_transform(mat: &SquareMatrix<i32>) -> SquareMatrix<f32> {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = mat.dimention;
    let mut vec = Vec::<f32>::with_capacity(mat.values.len());

    for v in 0..d {
        for u in 0..d {
            let index = v * d + u;

            let mut sum = 0f32;
            for y in 0..d {
                for x in 0..d {
                    let xy_index = y * d + x;
                    let gxy = mat.values[xy_index] as f32;

                    let prod = gxy * ((2f32 * x as f32 + 1f32) * u as f32 * Pi / 16f32).cos() *
                               ((2f32 * y as f32 + 1f32) * v as f32 * Pi / 16f32).cos();
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

fn main() {
    let mut matrix = sample_matrix();

    matrix.print();
    println!("");

    for a in matrix.values.iter_mut() {
        *a -= 127;
    }
    matrix.print();
    println!("");

    let mut transformed = discrete_cosinus_transform(&matrix);

    transformed.print();
    println!("");
}
