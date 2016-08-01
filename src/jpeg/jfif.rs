use std::iter::repeat;

use jpeg::huffman;
use ::transform;

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
            _ => {
                println!("wtf is unit {}? Default to NoUnits", byte);
                JFIFUnits::NoUnits
            }
            // _ => return Err(format!("Illegal unit byte: {}", byte)),
        })
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum JFIFVersion {
    V1_01,
    V1_02,
}

impl JFIFVersion {
    pub fn from_bytes(msb: u8, lsb: u8) -> Result<JFIFVersion, String> {
        Ok(match (msb, lsb) {
            (1, 1) => JFIFVersion::V1_01,
            (1, 2) => JFIFVersion::V1_02,
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
    huffman_ac_tables: [Option<huffman::Table>; 4],
    huffman_dc_tables: [Option<huffman::Table>; 4],
    quantization_tables: [Option<Vec<u8>>; 4],
    // TODO: multiple frames ?
    frame_header: Option<FrameHeader>,
}

#[derive(Debug)]
struct FrameHeader {
    /// Bits per sample of each component in the frame
    sample_precision: u8,
    /// The maximum number of lines in the source image
    num_lines: u16,
    /// The maximum number of samples per line in the source image
    samples_per_line: u16,
    /// Number of image components in the frame
    image_components: u8,
    /// Headers for each component
    frame_components: Vec<FrameComponentHeader>,
}

#[derive(Debug)]
struct FrameComponentHeader {
    /// Component id
    component_id: u8,
    /// Relationship between component horizontal dimension and maximum image dimension (?)
    horizontal_sampling_factor: u8,
    /// Relationship between component vertical dimension and maximum image dimension (?)
    vertical_sampling_factor: u8,
    /// Selector for this components quantization table
    quantization_selector: u8,
}

#[derive(Debug)]
struct ScanHeader {
    /// Number of components in the scan.
    num_components: u8,
    /// Headers for each component
    scan_components: Vec<ScanComponentHeader>,
    /// (?) Should be zero for seq. DCT
    start_spectral_selection: u8,
    /// (?) Should be 63 for seq. DCT
    end_spectral_selection: u8,
    /// Something something point transform
    successive_approximation_bit_pos_high: u8,
    /// Something something point transform
    successive_approximation_bit_pos_low: u8,
}

#[derive(Debug, Clone)]
struct ScanComponentHeader {
    /// Component id
    component_id: u8,
    /// Which DC huffman table the component uses
    dc_table_selector: u8,
    /// Which AC huffman table the component uses
    ac_table_selector: u8,
}

impl FrameHeader {
    fn quantization_table_id(&self, component: u8) -> Option<u8> {
        self.frame_components
            .iter()
            .find(|&comp_hdr| comp_hdr.component_id == component)
            .map(|ref comp_hdr| comp_hdr.quantization_selector)
    }
}

impl ScanHeader {
    fn scan_component_header(&self, component: u8) -> Option<ScanComponentHeader> {
        self.scan_components
            .iter()
            .find(|sch| sch.component_id == component)
            .map(|sch| sch.clone())
    }
}

#[derive(Debug)]
enum Marker {
    // TODO: fill in these
    StartOfScan,
    DefineHuffmanTable,
    Comment,
    QuantizationTable,
    BaselineDCT,
    RestartIntervalDefinition,
    ApplicationSegment12,
    ApplicationSegment14,
}

fn bytes_to_marker(data: &[u8]) -> Option<Marker> {
    if data[0] != 0xff {
        return None;
    }
    let mut n = data[1];
    if n == 0 {
        n = data[2];
    }
    use self::Marker::*;
    let marker = match n {
        0xc0 => BaselineDCT,
        0xc4 => DefineHuffmanTable,
        0xda => StartOfScan,
        0xdb => QuantizationTable,
        0xdd => RestartIntervalDefinition,
        0xec => ApplicationSegment12,
        0xee => ApplicationSegment14,
        0xfe => Comment,
        _ => return None,
    };
    Some(marker)
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
        let soi = 0xd8;
        let app_0 = 0xe0;
        if vec[0] != 0xff || vec[1] != soi || vec[2] != 0xff || vec[3] != app_0 ||
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

        let mut jfif_image = JFIFImage {
            version: version,
            units: units,
            dimensions: (x_density, y_density),
            thumbnail_dimensions: thumbnail_dimensions,
            comment: None,
            huffman_ac_tables: [None, None, None, None],
            huffman_dc_tables: [None, None, None, None],
            quantization_tables: [None, None, None, None],
            frame_header: None,
        };

        let bytes_to_len = |a: u8, b: u8| ((a as usize) << 8) + b as usize - 2;


        print_vector(vec.iter().skip(12152));

        let mut i = 20;
        'main_loop: while i < vec.len() {
            // All segments have a 2 byte length
            // right after the marker code
            let marker = bytes_to_marker(&vec[i..]);
            // NOTE: this does not count the length bytes anymore!
            // TODO: Maybe do count them? In order to make it less confusing
            let data_length = bytes_to_len(vec[i + 2], vec[i + 3]);

            if let Some(marker) = marker {
                match marker {
                    Marker::Comment => {
                        // Comment
                        use std::str;
                        let comment: String = match str::from_utf8(&vec[i + 4..i + 4 +
                                                                               data_length]) {
                            Ok(s) => s.to_string(),
                            Err(e) => {
                                println!("{}", e);
                                "".to_string()
                            }
                        };
                    }
                    Marker::QuantizationTable => {
                        // Quantization tables
                        // JPEG B.2.4.1

                        let mut index = i + 4;
                        while index < i + 4 + data_length {
                            let precision = (vec[index] & 0xf0) >> 4;
                            let identifier = vec[index] & 0x0f;

                            // TODO: we probably dont need to copy and collect here.
                            // Would rather have a slice in quant_tables, with a
                            // lifetime the same as jfif_image (?)
                            let table: Vec<u8> = vec[index + 1..]
                                .iter()
                                .take(64)
                                .map(|u| *u)
                                .collect();
                            jfif_image.quantization_tables[identifier as usize] = Some(table);
                            index += 65; // 64 entries + one header byte
                        }
                    }
                    Marker::BaselineDCT => {
                        // Baseline DCT
                        // JPEG B.2.2
                        let sample_precision = vec[i + 4];
                        let num_lines = u8s_to_u16(&vec[i + 5..]);
                        let samples_per_line = u8s_to_u16(&vec[i + 7..]);
                        let image_components = vec[i + 9];

                        let mut frame_components = Vec::with_capacity(image_components as usize);
                        let mut index = i + 10;
                        for component in 0..image_components {
                            let component_id = vec[index];
                            let horizontal_sampling_factor = (vec[index + 1] & 0xf0) >> 4;
                            let vertical_sampling_factor = vec[index + 1] & 0x0f;
                            let quantization_selector = vec[index + 2];

                            frame_components.push(FrameComponentHeader {
                                component_id: component_id,
                                horizontal_sampling_factor: horizontal_sampling_factor,
                                vertical_sampling_factor: vertical_sampling_factor,
                                quantization_selector: quantization_selector,
                            });
                            index += 3;
                        }
                        let frame_header = FrameHeader {
                            sample_precision: sample_precision,
                            num_lines: num_lines,
                            samples_per_line: samples_per_line,
                            image_components: image_components,
                            frame_components: frame_components,
                        };
                        jfif_image.dimensions = (samples_per_line, num_lines);
                        jfif_image.frame_header = Some(frame_header)
                    }
                    Marker::DefineHuffmanTable => {
                        // Define Huffman table
                        // JPEG B.2.4.2
                        // DC = 0, AC = 1

                        let mut huffman_index = i + 4;
                        let target_index = i + 4 + data_length;
                        // Read tables untill the segment is done

                        while huffman_index < target_index {
                            let table_class = (vec[huffman_index] & 0xf0) >> 4;
                            let table_dest_id = vec[huffman_index] & 0x0f;
                            huffman_index += 1;

                            // There are `size_area[i]` number of codes of length `i + 1`.
                            let size_area: &[u8] = &vec[huffman_index..huffman_index + 16];
                            huffman_index += 16;

                            let number_of_codes =
                                size_area.iter().fold(0u8, |a, b| a + *b) as usize;

                            // Code `i` has value `data_area[i]`
                            let data_area: &[u8] = &vec[huffman_index..huffman_index +
                                                                       number_of_codes];
                            huffman_index += number_of_codes;

                            let huffman_table = huffman::Table::from_size_data_tables(size_area,
                                                                                      data_area);
                            if table_class == 0 {
                                jfif_image.huffman_dc_tables[table_dest_id as usize] =
                                    Some(huffman_table);
                            } else {
                                jfif_image.huffman_ac_tables[table_dest_id as usize] =
                                    Some(huffman_table);
                            }
                        }
                        if huffman_index != target_index {
                            panic!("Read too much while parsing huffman tables! {}/{}",
                                   huffman_index,
                                   target_index);
                        }
                    }
                    Marker::StartOfScan => {
                        // Start of Scan
                        // JPEG B.2.3

                        let num_components = vec[i + 4];
                        let mut scan_components = Vec::new();
                        for component in 0..num_components {
                            scan_components.push(ScanComponentHeader {
                                component_id: vec[i + 5],
                                dc_table_selector: (vec[i + 6] & 0xf0) >> 4,
                                ac_table_selector: vec[i + 6] & 0x0f,
                            });
                            i += 2;
                        }

                        // TODO: Do we want to put the scan header in `FrameHeader`?
                        // We don't need it for simple decoding, but it might be useful
                        // if we want to print info (eg, all headers) for an image.
                        let scan_header = ScanHeader {
                            num_components: num_components,
                            scan_components: scan_components,
                            start_spectral_selection: vec[i + 5],
                            end_spectral_selection: vec[i + 6],
                            successive_approximation_bit_pos_high: (vec[i + 7] & 0xf0) >> 4,
                            successive_approximation_bit_pos_low: vec[i + 7] & 0x0f,
                        };
                        println!("{:#?}", scan_header);
                        i += 8;
                        // `i` is now at the head of the data.


                        // Try to find a marker:
                        let eos_index = {
                            let mut index = i;
                            while index < vec.len() - 1 {
                                let ff = vec[index];
                                let marker = vec[index + 1];
                                if ff == 0xff && marker != 0x00 {
                                    println!("Found marker at index {} : 0xff{:02x}",
                                             index,
                                             marker);
                                    break;
                                } else {
                                    index += 1;
                                }
                            }
                            index
                        };

                        // Copy data, and replace 0xff00 with 0xff.
                        let mut encoded_data = Vec::new();
                        {
                            let mut i = i;
                            while i < eos_index {
                                encoded_data.push(vec[i]);
                                if vec[i] == 0xff && vec[i + 1] == 0x00 {
                                    // Skip the 0x00 part here.
                                    i += 1;
                                }
                                i += 1;
                            }
                        }


                        // Map component_id to an index
                        let component_index_from_id = {
                            let map_component_to_i: Vec<u8> = scan_header.scan_components
                                .iter()
                                .map(|c| c.component_id)
                                .collect();
                            move |component: u8| {
                                map_component_to_i.iter()
                                    .enumerate()
                                    .find(|&(i, id)| *id == component)
                                    .map(|(i, id)| i as usize)
                            }
                        };
                        // Map index to component_id
                        let component_ids: Vec<u8> = scan_header.scan_components
                            .iter()
                            .map(|c| c.component_id)
                            .collect();

                        // For each component, create a Vec to hold the decoded blocks
                        let num_components = scan_header.num_components;
                        let mut blocks: Vec<Vec<Vec<i16>>> =
                            (0..num_components).map(|_| Vec::new()).collect();


                        // Step 1:
                        // Read all blocks.
                        let num_blocks_hori = (jfif_image.dimensions.0 as usize) / 8; // round up
                        let num_blocks_vert = (jfif_image.dimensions.1 as usize) / 8; // round up
                        let num_blocks = num_blocks_hori * num_blocks_vert;

                        let mut scan_state = huffman::ScanState {
                            index: 0,
                            bits_read: 0,
                        };
                        let iter_inspect = 9999;
                        for block_i in 0..num_blocks {
                            // Assume interleaved
                            for component in &scan_header.scan_components {
                                let component_id = component.component_id;

                                // Get tables
                                // TODO: Probably don't do this inside this loop.
                                //       Would rather move it outside, and use lookup
                                //       in here, with `component_id`.
                                let scan_component_header: ScanComponentHeader =
                                    scan_header.scan_component_header(component_id)
                                        .expect("ADD ERROR HANDLING");
                                let ac_index = scan_component_header.ac_table_selector as usize;
                                let ac_table = jfif_image.huffman_ac_tables[ac_index]
                                    .as_ref()
                                    .expect("ERROR FAIL xdd");
                                let dc_index = scan_component_header.dc_table_selector as usize;
                                let dc_table = jfif_image.huffman_dc_tables[dc_index]
                                    .as_ref()
                                    .expect("ERROR FAIL xdd");

                                let decoded = huffman::decode(ac_table,
                                                              dc_table,
                                                              &encoded_data[..],
                                                              &mut scan_state);

                                blocks[component_index_from_id(component_id).unwrap()]
                                    .push(decoded);
                            }
                        }

                        // Step 2:
                        // Do relative DC calculation, dequantization,
                        // and inverse DCT component for component
                        for component in &scan_header.scan_components {
                            let component_id = component.component_id;
                            let component_index = component_index_from_id(component_id)
                                .expect("ERROR");
                            let ref mut component_blocks = blocks[component_index];

                            // TODO: This is really ugly looking..
                            let quant_table_id = jfif_image.frame_header
                                .as_ref()
                                .unwrap()
                                .quantization_table_id(component_id)
                                .unwrap() as usize;
                            let quant_table = jfif_image.quantization_tables[quant_table_id]
                                .as_ref()
                                .unwrap();

                            let mut new_component_blocks = Vec::new();
                            let mut previous_dc = 0;
                            let mut iteration = 0;
                            for block in component_blocks.iter_mut() {
                                // DC correction
                                let encoded = block[0];
                                let decoded = encoded + previous_dc;
                                previous_dc = decoded;
                                block[0] = decoded;

                                // Dequantization, and convertion to f32
                                // We do this before `zigzag_inverse`, because
                                // the read quantization table is also zigzag.
                                let dequantized: Vec<f32> = block.iter()
                                    .zip(quant_table.iter())
                                    .map(|(&n, &q)| n as f32 * q as f32)
                                    .collect();

                                // TODO: Dequantization could take an iterator.
                                let dequantized = zigzag_inverse(&dequantized);

                                let spatial_block =
                                    transform::discrete_cosine_transform_inverse(&dequantized);

                                let color_values: Vec<_> = spatial_block.iter()
                                    // Skip rounding due to YCbCr
                                    .map(|n| (n + 128f32).round() as i16)
                                    .collect();

                                iteration += 1;
                                if iteration == iter_inspect {
                                    new_component_blocks.push(repeat(0).take(64).collect());
                                } else {
                                    new_component_blocks.push(color_values);
                                }
                            }
                            *component_blocks = new_component_blocks;
                        }

                        // Step 3:
                        // Merge the components. For now, assume YCbCr, and
                        // convert to RGB.

                        let mut rgb_blocks: Vec<Vec<(u8, u8, u8)>> = Vec::new();
                        if num_components == 3 {
                            let clamp_to_u8 = |num| if num > 255.0 {
                                255
                            } else if num < 0.0 {
                                0
                            } else {
                                num as u8
                            };
                            for block_i in 0..num_blocks {
                                let ref y_block = blocks[0][block_i];
                                let ref cb_block = blocks[1][block_i];
                                let ref cr_block = blocks[2][block_i];

                                let c_red: f32 = 0.299;
                                let c_green: f32 = 0.587;
                                let c_blue: f32 = 0.114;

                                let block: Vec<(u8, u8, u8)> = y_block.iter()
                                    .zip(cb_block.iter())
                                    .zip(cr_block.iter())
                                    .map(|((&y, &cb), &cr)| {
                                        (y as f32, cb as f32, cr as f32)
                                    })
                                    // .map(|(y, cb, cr)| {
                                    //     let r = cr * (2.0 - 2.0 * c_red) + y;
                                    //     let b = cb * (2.0 - 2.0 * c_blue) + y;
                                    //     let g = (y - c_blue * b - c_red * r) / c_green;
                                    //     println!("rgb({}, {}, {})", r, g, b);
                                    //     (r, g, b)
                                    // })
                                    .map(|(r, g, b)| (clamp_to_u8(r), clamp_to_u8(g), clamp_to_u8(b)))
                                    .collect();
                                rgb_blocks.push(block);
                            }
                        } else if num_components == 1 {
                            let clamp_to_u8 = |num| if num > 255 {
                                255
                            } else if num < 0 {
                                0
                            } else {
                                num as u8
                            };
                            for block in &blocks[0] {
                                rgb_blocks.push(block.iter()
                                    .map(|&c| (clamp_to_u8(c), clamp_to_u8(c), clamp_to_u8(c)))
                                    .collect());
                            }
                        }

                        // Step 4:
                        // Turn blocks into image lines, and put everything
                        // into one large array.

                        let mut image_data: Vec<(u8, u8, u8)> = Vec::with_capacity(num_blocks * 64);
                        for block_y in 0..num_blocks_vert {
                            for line in 0..8 {
                                for block_x in 0..num_blocks_hori {
                                    let block_index = num_blocks_hori * block_y + block_x;
                                    let ref block = rgb_blocks[block_index];
                                    for row in 0..8 {
                                        let in_block_index = 8 * line + row;
                                        image_data.push(block[in_block_index]);
                                    }
                                }
                            }
                        }

                        // Step 5:
                        // Show the image, somehow.

                        use std::fs::File;
                        use std::io::Write;
                        let mut file = File::create("output.ppm").unwrap();
                        let _ = file.write(format!("P3\n{} {}\n255\n",
                                                   8 * num_blocks_hori,
                                                   8 * num_blocks_vert)
                            .as_bytes());
                        for &(r, g, b) in &image_data {
                            let s = format!("{} {} {}\n", r, g, b);
                            let _ = file.write(s.as_bytes());
                        }
                    }
                    Marker::RestartIntervalDefinition => {
                        // Restart Interval Definition
                        // JPEG B.2.4.4
                        // TODO: support this
                        panic!("got to restart interval def")
                    }
                    Marker::ApplicationSegment12 => {
                        // Application segment 12
                        // Not to be found in the standard?
                        //
                        //      http://wooyaggo.tistory.com/104
                        //
                        // TODO: should clear this up.
                        panic!("got {:?}", marker);
                    }
                    Marker::ApplicationSegment14 => {
                        // Application segment 14
                        panic!("got {:?}", marker);
                    }
                }
            } else {
                println!("\n\nUnhandled byte marker: {:02x} {:02x}",
                         vec[i],
                         vec[i + 1]);
                println!("i = {}", i);
                println!("Total vector len = {}", vec.len());
                println!("len={}", data_length);
                print_vector(vec.iter().skip(i));
                break;
            }
            i += 4 + data_length;
        }
        Ok(jfif_image)
    }
}

// TODO: Remove (or move?)
use std::fmt::LowerHex;
#[allow(dead_code)]
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

use std::fmt::Binary;
#[allow(dead_code)]
fn print_vector_bin<I>(iter: I)
    where I: Iterator,
          I::Item: Binary
{
    let mut i = 0;
    for byte in iter.take(8) {
        i += 1;
        print!("{:08b} ", byte);
        if i % 8 == 0 && i != 0 {
            print!("\n");
        }
    }
    if i % 8 != 0 || i == 0 {
        print!("\n");
    }
}

use std::fmt::Display;
#[allow(dead_code)]
fn print_vector_dec<I>(iter: I)
    where I: Iterator,
          I::Item: Display
{
    let mut i = 0;
    for byte in iter.take(64) {
        i += 1;
        print!("{:8.2} ", byte);
        if i % 8 == 0 && i != 0 {
            print!("\n");
        }
    }
    if i % 8 != 0 || i == 0 {
        print!("\n");
    }
}

/// Turn a vector representing a Matrix into 'zigzag' order.
///
/// ```
///  0  1  2  3
///  4  5  6  7
///  8  9 10 11
/// 12 13 14 15
///
/// becomes
///
///  0  1  5  6
///  2  4  7 12
///  3  8 11 13
///  9 10 14 15
/// ```
///
// hardcode dis shit lol
const ZIGZAG_INDICES: [usize; 64] =
    [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27,
     20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58,
     59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63];
#[allow(dead_code)]
fn zigzag<T>(vec: &Vec<T>) -> Vec<T>
    where T: Copy
{
    if vec.len() != 64 {
        panic!("I took a shortcut in zigzag()! Please implement me properly :) (len={})",
               vec.len());
    }
    let mut res = Vec::with_capacity(64);
    for &i in ZIGZAG_INDICES.iter() {
        res.push(vec[i]);
    }
    res
}

#[allow(dead_code)]
fn zigzag_inverse<T>(vec: &Vec<T>) -> Vec<T>
    where T: Copy,
          T: Default
{
    if vec.len() != 64 {
        panic!("I took a shortcut in zigzag()! Please implement me properly :) (len={})",
               vec.len());
    }
    let mut res: Vec<T> = repeat(Default::default()).take(64).collect();
    for (i, &n) in ZIGZAG_INDICES.iter().enumerate() {
        res[n] = vec[i];
    }
    res
}
