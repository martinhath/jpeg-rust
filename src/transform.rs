use std::f32::consts::PI;

fn usize_square(n: usize) -> Option<usize> {
    let mut a = 1;
    while a * a < n {
        a += 1;
    }
    if a * a == n {
        Some(a)
    } else {
        None
    }
}

#[allow(non_upper_case_globals)]
const Pi: f32 = PI as f32;

pub fn discrete_cosine_transform(input: &[f32]) -> Vec<f32> {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = usize_square(input.len()).expect("Must supply a vector of square length!");
    let mut vec = Vec::with_capacity(input.len());

    for v in 0..d {
        for u in 0..d {
            let vf = v as f32;
            let uf = u as f32;

            let mut sum = 0f32;
            for y in 0..d {
                for x in 0..d {
                    let xy_index = y * d + x;
                    let gxy = input[xy_index] as f32;

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
    vec
}

pub fn discrete_cosine_transform_inverse(input: &[f32]) -> Vec<f32> {
    let alpha = |u| {
        if u == 0 {
            1f32 / 2f32.sqrt()
        } else {
            1f32
        }
    };
    let d = usize_square(input.len()).expect("Must supply a vector of square length!");
    let mut vec = Vec::with_capacity(input.len());

    for y in 0..d {
        for x in 0..d {
            let yf = y as f32;
            let xf = x as f32;
            let mut sum = 0f32;
            for v in 0..d {
                for u in 0..d {
                    let uf = u as f32;
                    let vf = v as f32;

                    let f_uv = input[v * d + u];
                    sum += alpha(u) * alpha(v) * f_uv *
                           ((2f32 * xf + 1f32) * uf * Pi / 16f32).cos() *
                           ((2f32 * yf + 1f32) * vf * Pi / 16f32).cos();
                }
            }
            vec.push(sum / 4f32);
        }
    }

    vec
}
