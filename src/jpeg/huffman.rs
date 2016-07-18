// Selects i bits, from msb to lsb.
const BIT_MASKS: [u16; 17] = [0x0, 0x8000, 0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00,
                              0xFF80, 0xFFC0, 0xFFE0, 0xFFF0, 0xFFF8, 0xFFFC, 0xFFFE, 0xFFFF];

#[derive(Debug)]
pub struct Table {
    code_table: Vec<u16>,
    code_vecs: [Vec<u8>; 16],
}

impl Table {
    /// Create a Huffman table from a size table and a corresponding data table.
    ///
    /// The size table describes how many codes there are of a given size. For
    /// each `i`, there are `size_table[i - 1]` codes, `1 <= i <= 16`.
    ///
    /// The data table describes the value of these codes. Code number `i` has
    /// the value `data_table[i]`.
    pub fn from_size_data_tables(size_data: &[u8], data_table: &[u8]) -> Table {
        // We use `id` to mark code number,
        // such that size_table maps code number to length of that code,
        // and code_table maps code number to code value.

        // id -> code length
        let size_table: Vec<u8> = Table::make_size_table(&size_data);
        // id -> 0b10101
        let code_table: Vec<u16> = Table::make_code_table(&size_table);
        // id -> 123

        // NOTE: TODO: this is really stupid.
        // Figure out something better!
        //
        // Make 16 vecs, one for each code length, containing
        // indices. When reading bytes, find the
        // first list containing the key.
        let mut code_vecs: [Vec<u8>; 16] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for i in 1..16 {
            let ref mut vec = code_vecs[i];
            for j in 0..size_table.len() {
                if size_table[j] == i as u8 {
                    vec.push(j as u8);
                }
            }
        }

        Table {
            code_table: code_table,
            code_vecs: code_vecs,
        }
    }

    /// Take a list of sizes, such that there are `bytes[i]` codes
    /// of size `i + 1`, and return a `Vec<u8>` of sizes such that
    /// code `i` is of size `vec[i]`.
    fn make_size_table(bytes: &[u8]) -> Vec<u8> {
        // See JPEG C.2
        // TODO: Check out LASTK ?
        let mut vec = Vec::new();
        for i in 0..16 {
            let num_codes_of_size = bytes[i] as usize;
            for _ in 0..num_codes_of_size {
                vec.push(i as u8 + 1);
            }
        }
        vec.push(0);
        vec
    }

    /// Take a size table, and return a `Vec<u16>` of codes,
    /// such that code `i` has the value `vec[i]`.
    fn make_code_table(size_table: &[u8]) -> Vec<u16> {
        let mut codes = Vec::new();

        let mut k = 0;
        let mut code: u16 = 0;
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

    pub fn decode(&self, data: &[u8]) -> Vec<u8> {
        // TODO: Clean up this!
        // TODO: We'll assume there is at least four bytes to read.
        let mut result = Vec::<u8>::new();
        let mut current = ((data[0] as u32) << 24) | ((data[1] as u32) << 16) |
                          ((data[3] as u32) << 8) |
                          ((data[3] as u32) << 0);
        // Index of next value to read
        let mut index = 4;
        // Number of bits shifted off current
        let mut bits_read = 0;

        'main: while index < data.len() {
            while bits_read >= 8 {
                current |= (data[index] as u32) << (bits_read - 8);
                bits_read -= 8;
                index += 1;
            }
            let current16 = ((current & 0xffff0000) >> 16) as u16;
            for length in 2..16 {
                let ref vec = self.code_vecs[length];

                let mask = BIT_MASKS[length];
                let code_candidate: u16 = (current16 & mask) >> (16 - length);

                // Loop over all ids which are of `length` length.
                for &id in vec.iter() {
                    let idu = id as usize;
                    let code = self.code_table[idu] as u16;
                    if code == code_candidate {
                        // success
                        let value = data[idu];
                        result.push(value);

                        current <<= length;
                        bits_read += length;
                        continue 'main;
                    }
                }
            }
            panic!("failed to find code for current: {:016b}", current);
        }
        result
    }

    /// Decode n bytes from `data`. Return the decoded bytes, as well
    /// as how many bytes from `data` was read.
    pub fn decode_n(&self, num: usize, data: &[u8]) -> (Vec<u8>, usize) {
        let mut result = Vec::<u8>::new();
        let mut bytes_decoded = 0;

        // Warning: mostly copied from decode().
        let mut current = ((data[0] as u32) << 24) | ((data[1] as u32) << 16) |
                          ((data[3] as u32) << 8) |
                          ((data[3] as u32) << 0);
        // Index of next value to read
        let mut index = 4;
        // Number of bits shifted off current
        let mut bits_read = 0;

        'main: while index < data.len() {
            while bits_read >= 8 {
                current |= (data[index] as u32) << (bits_read - 8);
                bits_read -= 8;
                index += 1;
            }
            let current16 = ((current & 0xffff0000) >> 16) as u16;
            for length in 2..16 {
                let ref vec = self.code_vecs[length];

                let mask = BIT_MASKS[length];
                let code_candidate: u16 = (current16 & mask) >> (16 - length);

                // Loop over all ids which are of `length` length.
                for &id in vec.iter() {
                    let idu = id as usize;
                    let code = self.code_table[idu] as u16;
                    if code == code_candidate {
                        // success
                        let value = data[idu];
                        result.push(value);
                        bytes_decoded += 1;
                        if bytes_decoded == num {
                            break 'main;
                        }

                        current <<= length;
                        bits_read += length;
                        continue 'main;
                    }
                }
            }
            panic!("failed to find code for current: {:016b}", current);
        }
        (result, index)
    }
}
