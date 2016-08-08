use jpeg::huffman;
use jpeg::decoder::JPEGDecoder;

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
    V1_02,
}

impl JFIFVersion {
    pub fn from_bytes(msb: u8, lsb: u8) -> Result<JFIFVersion, String> {
        Ok(match (msb, lsb) {
            (1, 1) => JFIFVersion::V1_01,
            (1, 2) => JFIFVersion::V1_02,
            _ => return Err(format!("Unknown version: ({}, {})", msb, lsb)),
        })
    }
}

type JPEGDimensions = (u16, u16);
type ThumbnailDimensions = (u8, u8);

/// Struct used to represent an image parsed by the library.
/// This should contain everything one would want to know
/// about the image.
#[derive(Debug)]
pub struct JFIFImage {
    /// JFIF version the image is compliant to
    version: JFIFVersion,
    /// TODO:
    units: JFIFUnits,
    /// Image dimensions
    dimensions: JPEGDimensions,
    /// Dimensions of the thumbnail, if present
    /// TODO: add thumbnail.
    /// Maybe join image data and dimensions to one struct?
    thumbnail_dimensions: Option<ThumbnailDimensions>,
    /// Optional comment
    comment: Option<String>,
    /// huffman tables for AC coefficients
    huffman_ac_tables: [Option<huffman::Table>; 4],
    /// huffman tables for DC coefficients
    huffman_dc_tables: [Option<huffman::Table>; 4],
    /// Quantization tables
    quantization_tables: [Option<Vec<u8>>; 4],
    /// Frame header data
    frame_header: Option<FrameHeader>,
    scan_headers: Vec<ScanHeader>,
    /// Actual image data.
    /// NOTE: only support 8-bit precision
    /// TODO: Add support for other precisions
    image_data: Option<Vec<(u8, u8, u8)>>,
}

#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Bits per sample of each component in the frame
    sample_precision: u8,
    /// The maximum number of lines in the source image
    num_lines: u16,
    /// The maximum number of samples per line in the source image
    samples_per_line: u16,
    /// Number of image components in the frame
    image_components: u8,
    /// Headers for each component
    pub frame_components: Vec<FrameComponentHeader>,
}

#[derive(Debug, Clone)]
pub struct FrameComponentHeader {
    /// Component id
    pub component_id: u8,
    /// Relationship between component horizontal dimension and maximum image dimension (?)
    pub horizontal_sampling_factor: u8,
    /// Relationship between component vertical dimension and maximum image dimension (?)
    pub vertical_sampling_factor: u8,
    /// Selector for this components quantization table
    pub quantization_selector: u8,
}

#[derive(Debug, Clone)]
pub struct ScanHeader {
    /// Number of components in the scan.
    num_components: u8,
    /// Headers for each component
    pub scan_components: Vec<ScanComponentHeader>,
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
pub struct ScanComponentHeader {
    /// Component id
    pub component_id: u8,
    /// Which DC huffman table the component uses
    pub dc_table_selector: u8,
    /// Which AC huffman table the component uses
    pub ac_table_selector: u8,
}

#[derive(Debug, PartialEq)]
enum Marker {
    // TODO: fill in the rest of these
    StartOfScan,
    DefineHuffmanTable,
    Comment,
    QuantizationTable,
    BaselineDCT,
    RestartIntervalDefinition,
    ApplicationSegment12,
    ApplicationSegment14,
    EndOfImage,
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
        0xd9 => EndOfImage,
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
    pub fn width(&self) -> usize {
        self.dimensions.0 as usize
    }

    pub fn height(&self) -> usize {
        self.dimensions.1 as usize
    }

    pub fn image_data(&self) -> Option<&Vec<(u8, u8, u8)>> {
        self.image_data.as_ref()
    }

    /// See if the bytes stream passed in is a JFIF file.
    fn recognize_jfif(bytes: &[u8]) -> bool {
        // you can identify a JFIF file by looking for the following sequence:
        //
        //      X'FF', SOI, X'FF', APP0, <2 bytes to be skipped>, "JFIF", X'00'.
        //
        // I'm not even sure which version is uglier: this,
        // or just checking `bytes[0] == 0xff && bytes[1] == ...`
        if bytes.len() < 11 {
            return false;
        }
        let jfif_format: [u8; 11] = [0xff, 0xd8, 0xff, 0xe0, 0, 0, 'J' as u8, 'F' as u8,
                                     'I' as u8, 'F' as u8, 0x00];
        let jfif_mask: [bool; 11] = [true, true, true, true, false, false, true, true, true, true,
                                     true];
        jfif_format.iter()
            .zip(jfif_mask.iter())
            .zip(bytes.iter())
            .all(|((&n, &include), &byte)| !include || n == byte)
    }

    pub fn parse(vec: Vec<u8>) -> Result<JFIFImage, String> {
        if vec.len() < 11 {
            return Err("input is too short".to_string());
        }
        if !JFIFImage::recognize_jfif(vec.as_slice()) {
            return Err("File is not a JPEG/JFIF file".to_string());
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
            thumbnail_dimensions: Some(thumbnail_dimensions),
            comment: None,
            huffman_ac_tables: [None, None, None, None],
            huffman_dc_tables: [None, None, None, None],
            quantization_tables: [None, None, None, None],
            frame_header: None,
            scan_headers: vec![],
            image_data: None,
        };

        let bytes_to_len = |a: u8, b: u8| ((a as usize) << 8) + b as usize - 2;

        let mut i = 20;
        while i < vec.len() {
            if let Some(marker) = bytes_to_marker(&vec[i..]) {
                if marker == Marker::EndOfImage {
                    // These are the last bytes, so it must be checked before
                    // `let data_length = ...` in order to avoid out-of-bounds indexes.
                    break;
                }

                // NOTE: this does not count the length bytes anymore!
                // TODO: Maybe do count them? In order to make it less confusing
                let data_length = ((vec[i + 2] as usize) << 8) + vec[i + 3] as usize - 2;

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
                            println!("Read too much while parsing huffman tables! {}/{}",
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
                        jfif_image.scan_headers.push(scan_header.clone());
                        i += 8;
                        // `i` is now at the head of the data.



                        // Try to find a marker:
                        let eos_index = {
                            let mut index = i;
                            while index < vec.len() - 1 {
                                let ff = vec[index];
                                let marker = vec[index + 1];
                                if ff == 0xff && marker == 0xd9 {
                                    break;
                                }
                                if ff == 0xff && marker != 0x00 {
                                    println!("Found marker at index {} : 0xff{:02x}",
                                             index,
                                             marker);
                                }
                                index += 1;
                            }
                            index
                        };


                        // Copy data, and replace 0xff00 with 0xff.
                        let mut bytes_skipped = 0;
                        let mut encoded_data = Vec::new();
                        {
                            let mut i = i;
                            while i < eos_index {
                                encoded_data.push(vec[i]);
                                if vec[i] == 0xff && vec[i + 1] == 0x00 {
                                    // Skip the 0x00 part here.
                                    i += 1;
                                    bytes_skipped += 1;
                                }
                                i += 1;
                            }
                        }


                        let frame_header = jfif_image.frame_header.clone().unwrap();
                        let mut jpeg_decoder = JPEGDecoder::new(encoded_data.as_slice())
                            .frame_header(frame_header)
                            // need `.clone()`, or we hit some LLVM bug??
                            .scan_header(scan_header.clone())
                            .dimensions((jfif_image.dimensions.0 as usize,
                                         jfif_image.dimensions.1 as usize));

                        // Add tables to `jpeg_decoder`
                        for (i, table) in jfif_image.huffman_ac_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.huffman_ac_tables(i as u8, table.clone());
                            }
                        }

                        for (i, table) in jfif_image.huffman_dc_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.huffman_dc_tables(i as u8, table.clone());
                            }
                        }

                        for (i, table) in jfif_image.quantization_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.quantization_table(i as u8, table.clone());
                            }
                        }

                        let (image_data, bytes_read) = jpeg_decoder.decode();
                        jfif_image.image_data = Some(image_data);

                        i += bytes_read + bytes_skipped;
                        continue;
                    }
                    Marker::RestartIntervalDefinition => {
                        // Restart Interval Definition
                        // JPEG B.2.4.4
                        // TODO: support this
                        println!("got to restart interval def")
                    }
                    Marker::ApplicationSegment12 => {
                        // Application segment 12
                        // Not to be found in the standard?
                        //
                        //      http://wooyaggo.tistory.com/104
                        //
                        // TODO: should clear this up.
                        println!("got {:?}", marker);
                    }
                    Marker::ApplicationSegment14 => {
                        // Application segment 14
                        println!("got {:?}", marker);
                    }
                    // Already handled
                    Marker::EndOfImage => {}
                }
                i += 4 + data_length;
            } else {
                println!("\n\nUnhandled byte marker: {:02x} {:02x}",
                         vec[i],
                         vec[i + 1]);
            }
        }
        Ok(jfif_image)
    }
}
