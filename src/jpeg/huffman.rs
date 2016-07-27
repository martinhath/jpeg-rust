use std::iter::repeat;
use std::cmp::min;

// Selects i bits, from msb to lsb.
const BIT_MASKS: [u16; 17] = [0x0, 0x8000, 0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00,
                              0xFF80, 0xFFC0, 0xFFE0, 0xFFF0, 0xFFF8, 0xFFFC, 0xFFFE, 0xFFFF];

#[derive(Debug)]
pub struct Table {
    /// ID -> code value lookup table.
    code_table: Vec<u16>,
    /// 16 vectors, one for each code length.
    /// All code IDs are saved in its vector.
    code_vecs: [Vec<u8>; 16],
    /// ID -> corresponding value
    data_table: Vec<u8>,
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
        let data_table = data_table;

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
            let ref mut vec = code_vecs[i - 1];
            for j in 0..size_table.len() {
                if size_table[j] == i as u8 {
                    vec.push(j as u8);
                }
            }
        }

        Table {
            code_table: code_table,
            code_vecs: code_vecs,
            data_table: data_table.iter().map(|u| *u).collect(),
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

    pub fn print_table(&self) {
        for (i, ref vec) in self.code_vecs.iter().enumerate() {
            let len = i + 1;
            for &id in vec.iter() {
                let code = self.code_table[id as usize];
                let code_string = format!("{:01$b}", code, len);
                println!("#{:3}\t{:3}\t{:>16}",
                         id,
                         self.data_table[id as usize],
                         code_string);
            }
        }
    }
}

use std::cell::Cell;
// TODO: Clean up this!
pub fn decode(ac_table: &Table, dc_table: &Table, data: &[u8]) -> (Vec<i16>, usize) {
    // TODO: For now, assume there is at least four bytes to read.

    // Stagety: `current` holds data from the data slice. The next data
    // to read is in the upper bits. After we have read `n` bits, we'll
    // shift `n` left, and get zeroes at the bottom. If we have shifted
    // more than 8 bits, we'll get a new number from `data`, and insert
    // it properly. This way, there will always be at least 25 readable
    // bits in `current`, on each new call to `get_next_code`.
    //
    // Return i16s, as coefficients before DCT may be large.
    // Also return the number of read elements from `data`, so the
    // caller know how far ahead to skip.
    //
    // TODO: what if `data` is empty, and we have the bits we need to
    //       finish in `current`?
    let current: Cell<u32> = Cell::new(((data[0] as u32) << 24) | ((data[1] as u32) << 16) |
                                       ((data[3] as u32) << 8) |
                                       ((data[3] as u32) << 0));

    // Number of bits shifted off current
    let bits_read = Cell::new(0);
    // Index of next value to read
    let index = Cell::new(4);

    let get_next_code = |table: &Table| -> u8 {
        // 16 upper bits of `current`
        let mut current16 = ((current.get() & 0xffff0000) >> 16) as u16;

        if current16 & 0xff00 == 0xff00 {
            let marker = current16 & 0x00ff;
            println!("Found marker 0xff{:02x} ({})", marker, bits_read.get());
            let length = 8;
            current.set(current.get() << length);
            bits_read.set(bits_read.get() + length);
            // Maybe shift in new bits from `data`
            while bits_read.get() >= 8 {
                current.set(current.get() |
                            (data[index.get() + 4] as u32) << (bits_read.get() - 8));
                bits_read.set(bits_read.get() - 8);
                index.set(index.get() + 1);
            }
            current16 = ((current.get() & 0xffff0000) >> 16) as u16;
        }
        // Check all code lengths, and try to find
        // a code that is the `length` upper bits of `current`.
        for length in 1..17 {
            let mask = BIT_MASKS[length];
            let code_candidate: u16 = ((current16 & mask) >> (16 - length)) as u16;

            for &id in table.code_vecs[length - 1].iter() {
                let idu = id as usize;
                let code = table.code_table[idu];
                if code == code_candidate {
                    // We found a valid code
                    let value = table.data_table[idu];
                    current.set(current.get() << length);
                    bits_read.set(bits_read.get() + length);
                    // Maybe shift in new bits from `data`
                    while bits_read.get() >= 8 {
                        current.set(current.get() |
                                    (data[index.get() + 4] as u32) << (bits_read.get() - 8));
                        bits_read.set(bits_read.get() - 8);
                        index.set(index.get() + 1);
                    }
                    return value;
                }
            }
        }
        panic!("failed to find code for current: {:16b}",
               (current.get() >> 16));
    };
    let read_n_bits = |n: u8| -> u32 {
        // TODO: implement properly
        if n > 16 {
            panic!("`BIT_MASKS` needs to be larger!");
            // If this is fixed, it is possible that we need to shift
            // in additonal numbers from `data` as well.
        }
        let current16 = current.get() >> 16;
        let mask = BIT_MASKS[n as usize] as u32;
        let number: u32 = (current16 & mask) >> (16 - n);
        current.set(current.get() << n);
        bits_read.set(bits_read.get() + n as usize);
        while bits_read.get() >= 8 {
            current.set(current.get() | (data[index.get() + 4] as u32) << (bits_read.get() - 8));
            bits_read.set(bits_read.get() - 8);
            index.set(index.get() + 1);
        }
        number
    };

    let dc_value_len = get_next_code(&dc_table);
    let dc_value = read_n_bits(dc_value_len);
    let dc_cof = dc_value_from_len_bits(dc_value_len, dc_value);

    let mut result = Vec::<i16>::new();
    result.push(dc_cof);

    let mut n_pushed: usize = 1;

    while n_pushed < 64 {
        if n_pushed != result.len() {
            panic!("wtf");
        }
        let next_code = get_next_code(&ac_table);
        if next_code == 0 {
            result.extend(repeat(0).take(64 - n_pushed));
            break;
        }
        if next_code == 0xf0 {
            panic!("GOT 0xf0");
        }
        let zeroes = ((next_code & 0xf0) >> 4) as usize;
        let num = next_code & 0x0f;
        for _ in 0..min(zeroes, 64 - n_pushed - 1) {
            result.push(0)
        }
        result.push(num as i16);
        n_pushed += (zeroes as usize) + 1;
    }

    if bits_read.get() > 0 {
        // Discard remaining bits in the half read byte
        index.set(index.get() + 1);
    }
    (result, index.get() - 4)
}

fn dc_value_from_len_bits(len: u8, bits: u32) -> i16 {
    // TODO: find out where this is in the standard.
    if len == 0 {
        return 0;
    }
    let bits = bits as i16;
    let base: i16 = 1 << (len - 1);
    if bits < base {
        -2 * base + 1 + bits
    } else {
        bits as i16
    }
}
