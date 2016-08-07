use std::iter::repeat;
use std::cmp::min;

// Selects i bits, from msb to lsb.
const BIT_MASKS: [u16; 17] = [0x0, 0x8000, 0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00,
                              0xFF80, 0xFFC0, 0xFFE0, 0xFFF0, 0xFFF8, 0xFFFC, 0xFFFE, 0xFFFF];

// TODO: Naming in this file is so bad..
// size_table? table? code_vecs? bah..
// PLEASE FIX!!


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

        for id in 0..code_table.len() {
            let length = size_table[id] as usize;
            let ref mut vec = code_vecs[length - 1];
            vec.push(id as u8);
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

    #[allow(dead_code)]
    pub fn print_table(&self) {
        for (i, ref vec) in self.code_vecs.iter().enumerate() {
            let len = i + 1;
            for &id in *vec {
                let code = self.code_table[id as usize];
                let code_string = format!("{:01$b}", code, len);
                println!("#{:3}\t{:3}\t{:<16}",
                         id,
                         self.data_table[id as usize],
                         code_string);
            }
        }
    }

    pub fn clone(&self) -> Table {
        let mut code_vecs: [Vec<u8>; 16] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                                            Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (i, v) in self.code_vecs.iter().enumerate() {
            code_vecs[i].extend(v);
        }
        Table {
            code_table: self.code_table.iter().cloned().collect(),
            code_vecs: code_vecs,
            data_table: self.data_table.iter().cloned().collect(),
        }
    }
}

pub struct HuffmanDecoder<'a> {
    /// Data stream
    data: &'a [u8],
    /// The index of next byte to read from the data stream
    next_index: usize,
    /// Number of bits read and shifted out of `current`.
    bits_read: usize,
    /// The bits we act on
    current: u32,
}

impl<'a> HuffmanDecoder<'a> {
    pub fn new(data: &'a [u8]) -> HuffmanDecoder<'a> {
        HuffmanDecoder {
            data: data,
            next_index: 0,
            bits_read: 0,
            current: 0,
        }
    }

    /// Read the next 8x8 block
    pub fn next_block(ac_table: &Table, dc_table: &Table) -> Vec<f32> {
        panic!("Implement me");
    }

    /// Read `n` bits from `current`
    fn read_n_bits(n: usize) -> u32 {
        panic!("Implement me");
    }

    /// Get the next code from `current` in the supplied table.
    fn next_code(table: &Table) {
        panic!("Implement me");
    }

    /// Get the next code from `current` in the supplied table, when
    /// we know the length of the code.
    fn next_code_n(len: usize) {
        panic!("Implement me");
    }

    /// Shift out `len` bits from `current`, and extend with new data
    /// from `data` if appropriate
    fn shift_and_fix_current(len: usize) {
        panic!("Implement me");
    }
}

#[derive(Debug)]
pub struct ScanState {
    pub index: usize,
    pub bits_read: usize,
}

use std::cell::Cell;
// TODO: Clean up this!
pub fn decode(ac_table: &Table,
              dc_table: &Table,
              data: &[u8],
              scan_state: &mut ScanState)
              -> Vec<i16> {
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
    //       finish in `current`? Probably not a problem, even though
    //       we will have non scan data in `current`.
    // TODO: naming - this is first illegal index.
    let last_index = data.len();

    // Need to check data[0-4] for 0xff bytes.
    // We might skip some bytes (eg when 0xff 0x00),
    // so this is the number of bytes actually read
    // (will usually be 4).
    let actual_n_bytes_read;
    // TODO: do this smarter!
    let current: Cell<u32> = Cell::new({
        let mut curr: u32 = 0;
        let mut i = scan_state.index;
        for it in 0..4 {
            let num = if i < last_index {
                data[i]
            } else {
                0xaa
            };
            // If we find 0xff 0x00 0x??, make it 0xff 0x??
            // This "solution" doesn't quite work, as
            // we may find the 0xff byte multiple times,
            // for instance we find it `i=3` first, then read
            // a whole byte, and find it again when `i=2`.
            if i < (last_index - 1) && data[i] == 0xff && data[i + 1] == 0x00 {
                i += 1;
            }

            let shift_length = 24 - it * 8;
            curr |= (num as u32) << shift_length;
            i += 1;
        }
        actual_n_bytes_read = i - scan_state.index;
        curr
    });

    // Number of bits shifted off current
    let bits_read = Cell::new(scan_state.bits_read);
    // Index of next value to read
    let index = Cell::new(scan_state.index + actual_n_bytes_read);

    if scan_state.bits_read > 0 {
        let new_current = current.get() << scan_state.bits_read;
        current.set(new_current);
    }

    let shift_and_fix_current = |n: usize| {
        // Shift out the upper `n` bits of current,
        // and update `index` and `bits_read`, reading
        // additional data from `data` if needed.
        current.set(current.get() << n);
        bits_read.set(bits_read.get() + n);
        // Maybe shift in new bits from `data`
        while bits_read.get() >= 8 {
            let i = index.get() as usize;
            let next_n = if i < last_index {
                data[i]
            } else {
                0xaa
            } as u32;
            current.set(current.get() | next_n << (bits_read.get() - 8));
            bits_read.set(bits_read.get() - 8);
            index.set(index.get() + 1);
        }
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
        shift_and_fix_current(n as usize);
        number
    };

    let get_next_code = |table: &Table| -> u8 {
        // 16 upper bits of `current`
        let current16 = ((current.get() & 0xffff0000) >> 16) as u16;
        // Check all code lengths, and try to find
        // a code that is the `length` upper bits of `current`.
        for length in 1..17 {
            let mask = BIT_MASKS[length];
            let code_candidate: u16 = ((current16 & mask) >> (16 - length)) as u16;

            for &id in &table.code_vecs[length - 1] {
                let idu = id as usize;
                let code = table.code_table[idu];
                if code == code_candidate {
                    // We found a valid code
                    let value = table.data_table[idu];
                    shift_and_fix_current(length);
                    return value;
                }
            }
        }
        println!("failed to find code for current: {:16b} (index: {}/{})",
                 current16,
                 index.get(),
                 last_index);
        0
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
        if next_code == 0x00 {
            // print!("(0, 0) ");
            result.extend(repeat(0).take(64 - n_pushed));
            break;
        }
        if next_code == 0xf0 {
            // print!("(f, 0) ");
            let num_to_push = min(16, 64 - n_pushed);
            result.extend(repeat(0).take(num_to_push));
            n_pushed += num_to_push;
            continue;
        }

        let zeroes = ((next_code & 0xf0) >> 4) as usize;
        let num_length = next_code & 0x0f;
        let num_bits = read_n_bits(num_length);
        let num = dc_value_from_len_bits(num_length, num_bits);
        // print!("({}, {})({}) ", zeroes, num_length, num);
        for _ in 0..min(zeroes, 64 - n_pushed - 1) {
            result.push(0)
        }
        result.push(num);
        n_pushed += (zeroes as usize) + 1;
    }

    let mut bits_read = bits_read.get();
    let mut index = index.get();

    // Normalize `index` and `bits_read`
    while bits_read >= 8 {
        index += 1;
        bits_read -= 8;
    }

    scan_state.index = index - 4;
    scan_state.bits_read = bits_read;
    // print!("\n");

    if result.len() != 64 {
        panic!("`result.len` should be 64");
    }

    result
}

fn dc_value_from_len_bits(len: u8, bits: u32) -> i16 {
    // See Table F.2
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
