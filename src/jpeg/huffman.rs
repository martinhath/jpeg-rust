use std::iter::repeat;
use std::iter;
use std::cmp::min;

// Selects i bits, from msb to lsb.
const BIT_MASKS: [u16; 17] = [0x0, 0x8000, 0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00,
                              0xFF80, 0xFFC0, 0xFFE0, 0xFFF0, 0xFFF8, 0xFFFC, 0xFFFE, 0xFFFF];

// TODO: Naming in this file is so bad..
// size_table? table? code_vecs? bah..
// PLEASE FIX!!

#[derive(Debug, Clone)]
pub struct Code {
    /// How many bits are used in the code
    length: u8,
    /// The bit code. If the number of bits used to represent the code is less
    /// than `length`, prepend `len-length` `0`s in front.
    code: u16,
    /// The value the code is mapped to.
    value: u8,
}

#[derive(Debug)]
pub struct Table {
    /// A list of all codes in the table, sorted on code length
    codes: Vec<Code>,
}

impl Table {
    /// Create a Huffman table from a size table and a corresponding data table.
    ///
    /// The size data describes how many codes there are of a given size. For
    /// each `i`, there are `size_table[i - 1]` codes, `1 <= i <= 16`.
    ///
    /// The data table describes the value of these codes. Code number `i` has
    /// the value `data_table[i]`.
    pub fn from_size_data_tables(size_data: &[u8], data_table: &[u8]) -> Table {
        // id -> code length
        let code_lengths: Vec<u8> = (0..16)
            .flat_map(|i| repeat(i as u8 + 1).take(size_data[i] as usize))
            .chain(iter::once(0))
            .collect();
        // id -> 0b10101
        let code_table: Vec<u16> = Table::make_code_table(&code_lengths);

        let codes: Vec<Code> = data_table.iter()
            .zip(code_lengths.iter())
            .zip(code_table.iter())
            .map(|((&value, &length), &code)| {
                Code {
                    length: length,
                    code: code,
                    value: value,
                }
            })
            .collect();

        Table { codes: codes }
    }

    pub fn codes_of_length(&self, len: usize) -> &[Code] {
        assert!(len >= 2);
        assert!(len < 17);
        let len_u8 = len as u8;
        let mut codes_of_length = self.codes
            .iter()
            .enumerate()
            .skip_while(|&(_, code)| code.length != len_u8)
            .take_while(|&(_, code)| code.length == len_u8);
        let (a, b) = if let Some(a) = codes_of_length.next().map(|(i, _)| i) {
            let b = codes_of_length.last().map(|(i, _)| i).unwrap_or(a) + 1;
            (a, b)
        } else {
            (0, 0)
        };
        &self.codes[a..b]
    }

    /// Take a size table, and return a `Vec<u16>` of codes,
    /// such that code `i` has the value `vec[i]`.
    fn make_code_table(sizes: &[u8]) -> Vec<u16> {
        // This is more or less just an implementation of a
        // flowchart (Figure C.2) in the standard.
        let mut vec = Vec::new();
        let mut code = 0;
        let mut current_size = sizes[0];
        for &size in sizes {
            while size > current_size {
                code <<= 1;
                current_size += 1;
            }
            vec.push(code);
            code += 1;
        }
        vec
    }
}

impl Clone for Table {
    fn clone(&self) -> Table {
        Table { codes: self.codes.iter().cloned().collect() }
    }
}

/// Struct used to handle state when decoding image blocks
/// encoded with huffman coding.
pub struct HuffmanDecoder<'a> {
    /// Data stream
    data: &'a [u8],
    /// The index of next byte to read from the data stream
    next_index: usize,
    /// Number of bits read and shifted out of `current`.
    bits_read: usize,
    /// The bits we act on.
    /// As codes might be 16 bits, we always need 16 readable
    /// bits in `current`. For simplicity, we'll keep 25-32
    /// readable bits in `current`.
    current: u32,
}

impl<'a> HuffmanDecoder<'a> {
    pub fn new(data: &'a [u8]) -> HuffmanDecoder<'a> {
        // TODO: Revisit this: is it weird to read from `data` in
        // the constructor?
        let current = ((data[0] as u32) << 24) | ((data[1] as u32) << 16) |
                      ((data[2] as u32) << 8) | ((data[3] as u32) << 0);
        HuffmanDecoder {
            data: data,
            next_index: 4,
            bits_read: 0,
            current: current,
        }
    }

    pub fn bits_read(&self) -> usize {
        self.bits_read
    }

    pub fn next_index(&self) -> usize {
        self.next_index
    }

    /// Read the next 8x8 block
    pub fn next_block(&mut self, ac_table: &Table, dc_table: &Table) -> Vec<i16> {
        // First we read the DC coefficient, which is encoded as
        // `(num_bits)(value)`, where `value` is _not_ huffman encoded,
        // but `num_bits` is.
        // TODO: remove `expect`
        let num_bits = self.next_code(dc_table);// .expect("Could not infer next code") as usize;
        if num_bits.is_none() {
            println!("DC lookup fail");
            println!("current_16 = {:016b}", (self.current >> 16) & 0xffff);
        }
        let num_bits = num_bits.unwrap() as usize;
        let dc_coef = HuffmanDecoder::value_correction(self.read_n_bits(num_bits), num_bits);

        let mut block: Vec<i16> = vec![dc_coef];

        while block.len() < 64 {
            let next_code = self.next_code(ac_table).expect("ILLEGAL STATE!");
            match next_code {
                0x00 => {
                    // End. Fill rest of `block` with `0`
                    let block_len = block.len();
                    block.extend(repeat(0).take(64 - block_len));
                    break;
                }
                0xf0 => {
                    // Push 16 `0`s
                    let to_push = min(16, 64 - block.len());
                    block.extend(repeat(0).take(to_push));
                    continue;
                }
                _ => {}
            }
            // The AC codes are laid out like this:
            // `(prepending_zeroes, num_bits_in_code)(code)`
            // where `prepending_zeroes` and `num_bits` are 4 bits, and
            // `code` is `num_bits` long.
            // The tuple is huffman encoded. `code` is not.
            let prepending_zeroes = ((next_code & 0xf0) >> 4) as usize;
            let num_bits = (next_code & 0xf) as usize;
            let num = self.read_n_bits(num_bits);
            let number = HuffmanDecoder::value_correction(num, num_bits);
            let zeroes_to_push = min(prepending_zeroes, 64 - block.len() - 1);
            block.extend(repeat(0).take(zeroes_to_push));
            block.push(number);
        }

        assert!(block.len() == 64);

        block
    }

    /// Read `n` bits from `current`
    fn read_n_bits(&mut self, n: usize) -> u16 {
        if n == 0 {
            return 0;
        }
        assert!(n <= 16, "Should not read more than 16 bits at a time!");
        let mask = BIT_MASKS[n];
        let current_16 = (self.current >> 16) as u16;
        let number = ((current_16 & mask) >> (16 - n)) as u16;
        self.shift_and_fix_current(n);
        number
    }

    /// Get the next code from `current` in the supplied table.
    fn next_code(&mut self, table: &Table) -> Option<u8> {
        (2..17)
            .flat_map(|len| {
                let mask = BIT_MASKS[len];
                let current_16 = (self.current >> 16) as u16;
                let bits = ((current_16 & mask) >> (16 - len)) as u16;

                table.codes_of_length(len).iter()
                    // Find the code ID of length `len`, and code bits `bits`
                    .find(|&code| code.code == bits)
                    .map(|code| {
                        self.shift_and_fix_current(len);
                        code.value
                    })
            })
            .nth(0)
    }

    /// Shift out `len` bits from `current`, and extend with new data
    /// from `self.data` if appropriate
    fn shift_and_fix_current(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.current <<= len;
        self.bits_read += len;
        while self.bits_read >= 8 {
            self.bits_read -= 8;
            let next_num = {
                if self.next_index >= self.data.len() {
                    // We might need to shift in additional data
                    // when we are at the end. Assuming the file
                    // is well formed, these values will not be read,
                    // and is only fill data, in order to avoid
                    // out-of-range indexing.
                    0xaa
                } else {
                    self.data[self.next_index]
                }
            } as u32;
            self.current |= next_num << self.bits_read;
            self.next_index += 1;
        }
    }

    fn value_correction(val: u16, len: usize) -> i16 {
        // See Table F.2 in the JPEG Standard
        if len == 0 {
            return 0;
        }
        let val = val as i16;
        let base: i16 = 1 << (len - 1);
        if val < base {
            -2 * base + 1 + val
        } else {
            val as i16
        }
    }
}
