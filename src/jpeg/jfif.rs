use jpeg::huffman;
use std::iter;

// TODO: move this?
fn u8s_to_u16(bytes: &[u8]) -> u16 {
    let msb = bytes[0] as u16;
    let lsb = bytes[1] as u16;
    (msb << 8) + lsb
}


#[derive(Debug)]
pub enum JFIFUnits {
    NoUnits,
    DotsPerInch,
    DotsPerCm,
}

impl JFIFUnits {
    pub fn from_u8(byte: u8) -> Result<JFIFUnits, String> {
        Ok(match byte {
            1 => JFIFUnits::NoUnits,
            2 => JFIFUnits::DotsPerInch,
            3 => JFIFUnits::DotsPerCm,
            _ => return Err(format!("Illegal unit byte: {}", byte)),
        })
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum JFIFVersion {
    V1_01,
}

impl JFIFVersion {
    pub fn from_bytes(msb: u8, lsb: u8) -> Result<JFIFVersion, String> {
        Ok(match (msb, lsb) {
            (1, 1) => JFIFVersion::V1_01,
            _ => return Err(format!("Illegal version: ({}, {})", msb, lsb)),
        })
    }
}

type JPEGDimensions = (u16, u16);
type ThumbnailDimensions = (u8, u8);

#[derive(Debug)]
pub struct JFIFImage {
    version: JFIFVersion,
    units: JFIFUnits,
    dimensions: JPEGDimensions,
    thumbnail_dimensions: ThumbnailDimensions,
    comment: Option<String>,
    huffman_tables: Vec<Option<huffman::Table>>,

    // tmp
    data_index: usize,
    raw_data: Vec<u8>, // TOOD: add all options, such as progressive/sequential, etc.
}

#[allow(unused_variables)]
impl JFIFImage {
    pub fn parse(vec: Vec<u8>) -> Result<JFIFImage, String> {
        // you can identify a JFIF file by looking for the following sequence:
        //
        //      X'FF', SOI, X'FF', APP0, <2 bytes to be skipped>, "JFIF", X'00'.
        if vec.len() < 11 {
            return Err("input is too short".to_string());
        }
        let SOI = 0xd8;
        let APP0 = 0xe0;
        if vec[0] != 0xff || vec[1] != SOI || vec[2] != 0xff || vec[3] != APP0 ||
           vec[6] != 'J' as u8 || vec[7] != 'F' as u8 || vec[8] != 'I' as u8 ||
           vec[9] != 'F' as u8 || vec[10] != 0x00 {
            return Err("Header mismatch".to_string());
        }
        let version = try!(JFIFVersion::from_bytes(vec[11], vec[12]));

        let units = try!(JFIFUnits::from_u8(vec[13]));
        let x_density = u8s_to_u16(&vec[14..16]);
        let y_density = u8s_to_u16(&vec[16..18]);
        let thumbnail_dimensions = (vec[18], vec[19]);

        // TODO: thumbnail data?
        // let n = thumbnail_dimensions.0 as usize * thumbnail_dimensions.1 as usize;

        let mut huffman_tables = Vec::with_capacity(16);
        for _ in 0..16 {
            huffman_tables.push(None);
        }
        let mut jfif_image = JFIFImage {
            version: version,
            units: units,
            dimensions: (x_density, y_density),
            thumbnail_dimensions: thumbnail_dimensions,
            huffman_tables: huffman_tables,

            comment: None,

            data_index: 0,
            raw_data: Vec::new(),
        };

        let bytes_to_len = |a: u8, b: u8| ((a as usize) << 8) + b as usize - 2;

        let mut i = 20;
        loop {
            // All segments have a 2 byte length
            // right after the marker code
            let data_length = bytes_to_len(vec[i + 2], vec[i + 3]);
            match (vec[i], vec[i + 1]) {
                (0xff, 0xfe) => {
                    // Comment
                    use std::str;
                    let comment: String = match str::from_utf8(&vec[i + 4..i + 4 + data_length]) {
                        Ok(s) => s.to_string(),
                        Err(e) => {
                            println!("{}", e);
                            "".to_string()
                        }
                    };
                    // println!("found comment '{}'", comment);
                }
                (0xff, 0xdb) => {
                    // Quantization tables
                    // JPEG B.2.4.1

                    let p_q = (vec[i + 4] & 0xf0) >> 4;
                    let t_q = vec[i + 4] & 0x0f;
                    let quant_values = &vec[i + 5..i + 4 + data_length];

                    // Do whatever
                }
                (0xff, 0xc0) => {
                    // Baseline DCT
                    // JPEG B.2.2

                    // TODO: Make use of this
                }
                (0xff, 0xc4) => {
                    // Define Huffman table
                    // JPEG B.2.4.2
                    let table_class = (vec[i + 4] & 0xf0) >> 4;
                    let table_dest_id = vec[i + 4] & 0x0f;
                    // println!("Huffman table: len: {}\tclass: {}\tdest_id: {}",
                    //          data_length,
                    //          table_class,
                    //          table_dest_id);

                    // There are size_area[i] number of codes of length i + 1.
                    let size_area: &[u8] = &vec[i + 5..i + 5 + 16];
                    // Code i has value data_area[i]
                    let data_area: &[u8] = &vec[i + 5 + 16..i + 4 + data_length];
                    let huffman_table = huffman::Table::from_size_data_tables(size_area, data_area);
                    jfif_image.huffman_tables.insert(table_dest_id as usize, Some(huffman_table));
                }
                (0xff, 0xda) => {
                    // Start of Scan
                    // JPEG B.2.3
                    let num_components = vec[i + 4];

                    i += 2 * num_components as usize;
                    let start_spectral_section = vec[i + 5];
                    let end_spectral_section = vec[i + 6];
                    let al_ah = vec[i + 7];

                    println!("start spectral section={}", start_spectral_section);
                    println!("end spectral section={}", end_spectral_section);

                    // After the scan header is parsed, we start to read data.
                    // See Figure B.2 in B.2.1
                    print_vector(vec.iter().skip(i + 8));
                    // How long is the ECS?? Maybe it can be derived
                    // from values read in some headers.
                    // If we are not using `restart`, there is only one segment.
                    let data_length = end_spectral_section - start_spectral_section + 1;
                    // Read `data_length`  values into `frequencies`
                    let mut frequencies = Vec::<u8>::with_capacity(data_length as usize);
                    let mut index = 0;
                    while index < data_length {
                    }

                    i += data_length as usize;
                }
                (0xff, 0xdd) => {
                    // Restart Interval Definition
                    // JPEG B.2.4.4
                    panic!("got to restart interval def")
                }
                _ => {
                    println!("\n\nUnhandled byte marker: {:02x} {:02x}",
                             vec[i],
                             vec[i + 1]);
                    println!("len={}", data_length);
                    print_vector(vec.iter().skip(i));
                    break;
                }
            }
            i += 4 + data_length;
        }
        panic!();
        // Ok(jfif_image)
    }

    pub fn get_nth_square(&self, n: usize) -> &[u8] {
        // let square_size = self.dimensions.0 as usize * self.dimensions.1 as usize;
        let square_size = 8 * 8;
        let a = self.data_index + square_size * n;
        let b = a + square_size;
        &self.raw_data[a..b]
    }
}

// TODO: Remove (or move?)
use std::fmt::LowerHex;
fn print_vector<I>(iter: I)
    where I: Iterator,
          I::Item: LowerHex
{
    let mut i = 0;
    for byte in iter.take(128) {
        i += 1;
        print!("{:02x} ", byte);
        if i % 16 == 0 && i != 0 {
            print!("\n");
        }
    }
    if i % 16 != 0 || i == 0 {
        print!("\n");
    }
}
