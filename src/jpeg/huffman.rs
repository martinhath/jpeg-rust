
pub fn make_size_table(bytes: &[u8]) -> Vec<u8> {
    let mut vec = Vec::new();
    for i in 0..16 {
        let num_codes_of_size = bytes[i] as usize;
        for _ in 0..num_codes_of_size {
            vec.push(i as u8 + 1);
        }
    }
    vec.push(0);
    println!("{:?}", vec);
    vec
}

pub fn make_code_table(size_table: &Vec<u8>) -> Vec<usize> {
    let mut codes = Vec::new();

    let mut k = 0;
    let mut code = 0;
    let mut si = size_table[0] as usize;

    loop {
        codes.push(code);
        code += 1;
        k += 1;

        let size_k = size_table[k] as usize;
        if size_k == si {
            continue;
        }

        if size_k == 0 {
            break;
        }

        // NOTE: this is a do-while loop :)
        while {
            code = code << 1;
            si += 1;
            size_k != si
        } {}
    }


    codes
}
