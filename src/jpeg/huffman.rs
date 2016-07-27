use std::iter::repeat;

// Selects i bits, from msb to lsb.
const BIT_MASKS: [u16; 17] = [0x0, 0x8000, 0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00,
                              0xFF80, 0xFFC0, 0xFFE0, 0xFFF0, 0xFFF8, 0xFFFC, 0xFFFE, 0xFFFF];

#[derive(Debug)]
pub struct Table {
    /// ID -> value lookup table.
    code_table: Vec<u16>,
    /// 16 vectors, one for each code length.
    /// All code IDs are saved in its vector.
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
}

#[derive(Debug)]
pub struct ReadState {
    pub index: usize,
    pub bits_read: usize,
}

impl ReadState {
    pub fn new() -> ReadState {
        ReadState {
            index: 0,
            bits_read: 0,
        }
    }
}

use std::ops::AddAssign;
impl<'a> AddAssign<&'a ReadState> for ReadState {
    fn add_assign(&mut self, rhs: &'a ReadState) {
        self.index = rhs.index;
        self.bits_read += rhs.bits_read;
        while self.bits_read >= 8 {
            self.index += 1;
            self.bits_read -= 8;
        }
    }
}

use std::cell::Cell;
// TODO: Clean up this!
pub fn decode(ac_table: &Table,
              dc_table: &Table,
              data: &[u8],
              read_state: Option<ReadState>)
              -> (Vec<i16>, ReadState) {
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
    let mut result = Vec::<i16>::new();

    // Number of bits shifted off current
    let bits_read;
    // Index of next value to read
    let index;
    match read_state {
        Some(read_state) => {
            index = Cell::new(read_state.index);
            bits_read = Cell::new(read_state.bits_read);
        }
        None => {
            index = Cell::new(0);
            bits_read = Cell::new(0);
        }
    }
    let current: Cell<u32> = Cell::new(((data[index.get() + 0] as u32) << 24) |
                                       ((data[index.get() + 1] as u32) << 16) |
                                       ((data[index.get() + 3] as u32) << 8) |
                                       ((data[index.get() + 3] as u32) << 0));

    let get_next_code = |table: &Table| -> u8 {
        // 16 upper bits of `current`
        let current16 = ((current.get() & 0xffff0000) >> 16) as u16;
        // Check all code lengths, and try to find
        // a code that is the `length` upper bits of `current`.
        for length in 2..16 {
            let mask = BIT_MASKS[length];
            let code_candidate: u8 = ((current16 & mask) >> (16 - length)) as u8;

            // Look for `code_candidate`
            if table.code_vecs[length].iter().any(|&id| {
                let code = table.code_table[id as usize] as u8;
                code == code_candidate
            }) {
                // Shift out the bits we just read
                current.set(current.get() << length);
                bits_read.set(bits_read.get() + length);
                // Maybe shift in new bits from `data`
                while bits_read.get() >= 8 {
                    current.set(current.get() |
                                (data[index.get() + 4] as u32) << (bits_read.get() - 8));
                    bits_read.set(bits_read.get() - 8);
                    index.set(index.get() + 1);
                }
                return code_candidate;
            }
        }
        panic!("failed to find code for current: {:016b}", current.get());
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
    let mut dc_cof = dc_value_from_len_bits(dc_value_len, dc_value);

    result.push(dc_cof);

    let mut n_pushed = 1;

    while n_pushed < 64 {
        let next_code = get_next_code(&ac_table);
        if next_code == 0 {
            result.extend(repeat(0).take(64 - n_pushed));
            break;
        }
        let zeroes = (next_code & 0xf0) >> 4;
        let num = next_code & 0x0f;
        for _ in 0..zeroes {
            result.push(0)
        }
        result.push(num as i16);
        n_pushed += (zeroes as usize) + 1;
    }

    (result,
     ReadState {
        index: index.get(),
        bits_read: bits_read.get(),
    })
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
