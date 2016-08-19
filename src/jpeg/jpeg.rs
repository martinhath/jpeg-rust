use jpeg::huffman;
use jpeg::decoder::JPEGDecoder;

use std::str;

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
    Unknown(u8),
}

impl JFIFUnits {
    pub fn from_u8(byte: u8) -> JFIFUnits {
        match byte {
            1 => JFIFUnits::NoUnits,
            2 => JFIFUnits::DotsPerInch,
            3 => JFIFUnits::DotsPerCm,
            _ => JFIFUnits::Unknown(byte),
        }
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum JFIFVersion {
    V1_01,
    V1_02,
    Unknown(u8, u8),
}

impl JFIFVersion {
    pub fn from_bytes(msb: u8, lsb: u8) -> JFIFVersion {
        match (msb, lsb) {
            (1, 1) => JFIFVersion::V1_01,
            (1, 2) => JFIFVersion::V1_02,
            _ => JFIFVersion::Unknown(msb, lsb),
        }
    }
}

type JPEGDimensions = (u16, u16);
type ThumbnailDimensions = (u8, u8);

/// Struct used to represent an image parsed by the library.
/// This should contain everything one would want to know
/// about the image.
#[derive(Debug)]
pub struct JPEGImage {
    /// JFIF version the image is compliant to
    version: JFIFVersion,
    /// TODO:
    units: JFIFUnits,
    /// TODO:
    pixel_density: (u16, u16),
    /// Image dimensions
    dimensions: JPEGDimensions,
    /// Dimensions of the thumbnail, if present
    /// TODO: add thumbnail.
    /// Maybe join image data and dimensions to one struct?
    thumbnail_dimensions: ThumbnailDimensions,
    /// Optional comment
    comment: Option<String>,
    /// huffman tables for AC coefficients
    huffman_ac_tables: [Option<huffman::HuffmanTable>; 4],
    /// huffman tables for DC coefficients
    huffman_dc_tables: [Option<huffman::HuffmanTable>; 4],
    /// Quantization tables
    quantization_tables: [Option<Vec<u8>>; 4],
    /// Frame header data
    frame_header: Option<FrameHeader>,
    scan_headers: Option<Vec<ScanHeader>>,
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
    ApplicationSegment0,
    ApplicationSegment12,
    ApplicationSegment14,
    StartOfImage,
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
        0xd8 => StartOfImage,
        0xd9 => EndOfImage,
        0xda => StartOfScan,
        0xdb => QuantizationTable,
        0xdd => RestartIntervalDefinition,
        0xe0 => ApplicationSegment0,
        0xec => ApplicationSegment12,
        0xee => ApplicationSegment14,
        0xfe => Comment,
        _ => return None,
    };
    Some(marker)
}

#[allow(unused_variables)]
impl JPEGImage {
    fn new() -> JPEGImage {
        JPEGImage {
            version: JFIFVersion::Unknown(0, 0),
            units: JFIFUnits::Unknown(0),
            pixel_density: (0, 0),
            dimensions: (0, 0),
            thumbnail_dimensions: (0, 0),
            comment: None,
            huffman_ac_tables: [None, None, None, None],
            huffman_dc_tables: [None, None, None, None],
            quantization_tables: [None, None, None, None],
            frame_header: None,
            scan_headers: None,
            image_data: None,
        }
    }

    pub fn parse(vec: Vec<u8>) -> Result<JPEGImage, String> {
        let mut image = JPEGImage::new();

        let mut i = 0;
        while i < vec.len() {
            if let Some(marker) = bytes_to_marker(&vec[i..]) {
                if marker == Marker::EndOfImage || marker == Marker::StartOfImage {
                    // These markers doesn't have length bytes, so they must be
                    // handled separately, in order to to avoid out-of-bounds indexes,
                    // or reading nonsense lengths.
                    i += 2;
                    continue;
                }

                // NOTE: this does not count the length bytes anymore!
                // TODO: Maybe do count them? In order to make it less confusing
                let data_length = (u8s_to_u16(&vec[i + 2..]) - 2) as usize;
                i += 4;

                match marker {
                    Marker::Comment => {
                        let comment = str::from_utf8(&vec[i..i + data_length])
                            .map(|s| s.to_string())
                            .ok();
                        image.comment = comment;
                    }
                    Marker::QuantizationTable => {
                        // JPEG B.2.4.1
                        let mut index = i;
                        while index < i + data_length {
                            let precision = (vec[index] & 0xf0) >> 4;
                            assert!(precision == 0);
                            let identifier = vec[index] & 0x0f;
                            let table: Vec<u8> = vec[index + 1..index + 65]
                                .iter()
                                .cloned()
                                .collect();

                            image.quantization_tables[identifier as usize] = Some(table);
                            index += 65; // 64 entries + one header byte
                        }
                    }
                    Marker::BaselineDCT => {
                        // JPEG B.2.2
                        let sample_precision = vec[i];
                        let num_lines = u8s_to_u16(&vec[i + 1..]);
                        let samples_per_line = u8s_to_u16(&vec[i + 3..]);
                        let image_components = vec[i + 5];

                        let mut frame_components = Vec::with_capacity(image_components as usize);
                        let mut index = i + 6;
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
                        image.dimensions = (samples_per_line, num_lines);
                        image.frame_header = Some(frame_header)
                    }
                    Marker::DefineHuffmanTable => {
                        // JPEG B.2.4.2

                        // Head of data for each table
                        let mut huffman_index = i;
                        // End of segment
                        let segment_end = i + data_length;

                        while huffman_index < segment_end {
                            let table_class = (vec[huffman_index] & 0xf0) >> 4;
                            let table_dest_id = vec[huffman_index] & 0x0f;
                            huffman_index += 1;

                            // There are `size_area[i]` number of codes of length `i + 1`.
                            let size_area: &[u8] = &vec[huffman_index..huffman_index + 16];
                            huffman_index += 16;

                            // TODO: replace with `.sum` as of Rust 1.11
                            let number_of_codes = size_area.iter()
                                .fold(0, |a, b| a + (*b as usize));

                            // Code `i` has value `data_area[i]`
                            let data_area: &[u8] = &vec[huffman_index..huffman_index +
                                                                       number_of_codes];
                            huffman_index += number_of_codes;

                            let huffman_table =
                                huffman::HuffmanTable::from_size_data_tables(size_area, data_area);
                            // DC = 0, AC = 1
                            if table_class == 0 {
                                image.huffman_dc_tables[table_dest_id as usize] =
                                    Some(huffman_table);
                            } else {
                                image.huffman_ac_tables[table_dest_id as usize] =
                                    Some(huffman_table);
                            }
                        }
                    }
                    Marker::StartOfScan => {
                        // JPEG B.2.3
                        let num_components = vec[i];
                        let mut scan_components = Vec::new();
                        for component in 0..num_components {
                            scan_components.push(ScanComponentHeader {
                                component_id: vec[i + 1],
                                dc_table_selector: (vec[i + 2] & 0xf0) >> 4,
                                ac_table_selector: vec[i + 2] & 0x0f,
                            });
                            i += 2;
                        }

                        // TODO: Do we want to put the scan header in `FrameHeader`?
                        // We don't need it for simple decoding, but it might be useful
                        // if we want to print info (eg, all headers) for an image.
                        let scan_header = ScanHeader {
                            num_components: num_components,
                            scan_components: scan_components,
                            start_spectral_selection: vec[i + 1],
                            end_spectral_selection: vec[i + 2],
                            successive_approximation_bit_pos_high: (vec[i + 3] & 0xf0) >> 4,
                            successive_approximation_bit_pos_low: vec[i + 3] & 0x0f,
                        };
                        // Register read data
                        i += 4;

                        if image.scan_headers.is_none() {
                            image.scan_headers = Some(Vec::new());
                        }
                        image.scan_headers
                            .as_mut()
                            .map(|v| v.push(scan_header.clone()));

                        // Copy data, and replace 0xff00 with 0xff.
                        let mut bytes_skipped = 0;
                        let mut encoded_data = Vec::new();
                        {
                            let mut i = i;
                            while i < vec.len() {
                                encoded_data.push(vec[i]);
                                if vec[i] == 0xff && vec[i + 1] == 0x00 {
                                    // Skip the 0x00 part here.
                                    i += 1;
                                    bytes_skipped += 1;
                                }
                                i += 1;
                            }
                        }


                        let frame_header = image.frame_header.clone().unwrap();
                        let mut jpeg_decoder = JPEGDecoder::new(encoded_data.as_slice())
                            .frame_header(frame_header)
                            // need `.clone()`, or we hit some LLVM bug??
                            .scan_header(scan_header.clone())
                            .dimensions((image.dimensions.0 as usize,
                                         image.dimensions.1 as usize));

                        // Add tables to `jpeg_decoder`
                        for (i, table) in image.huffman_ac_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.huffman_ac_tables(i as u8, table.clone());
                            }
                        }

                        for (i, table) in image.huffman_dc_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.huffman_dc_tables(i as u8, table.clone());
                            }
                        }

                        for (i, table) in image.quantization_tables.iter().enumerate() {
                            if let &Some(ref table) = table {
                                jpeg_decoder.quantization_table(i as u8, table.clone());
                            }
                        }

                        let (image_data, bytes_read) = jpeg_decoder.decode();
                        image.image_data = Some(image_data);

                        // Since we are calculating how much data there is in this segment,
                        // we update `i` manually, and `continue` the `while` loop.
                        i += bytes_read + bytes_skipped;
                        continue;
                    }
                    Marker::RestartIntervalDefinition => {
                        // JPEG B.2.4.4
                        // TODO: support this
                        panic!("got to restart interval def")
                    }
                    Marker::ApplicationSegment0 => {
                        // JFIF puts stuff here.
                        //
                        //
                        //  X’FF’, APP0, length, identifier, version, units,
                        //  Xdensity, Ydensity, Xthumbnail, Ythumbnail, (RGB)n

                        let identifier = &vec[i..i + 6];
                        let version = JFIFVersion::from_bytes(vec[7], vec[8]);
                        let units = JFIFUnits::from_u8(vec[13]);

                        let x_density = u8s_to_u16(&vec[10..12]);
                        let y_density = u8s_to_u16(&vec[12..14]);

                        let thumbnail_dimensions = (vec[14], vec[15]);
                    }
                    Marker::ApplicationSegment12 => {
                        panic!("got {:?}", marker);
                    }
                    Marker::ApplicationSegment14 => {
                        panic!("got {:?}", marker);
                    }
                    // Already handled
                    Marker::StartOfImage => {}
                    Marker::EndOfImage => {}
                }
                i += data_length;
            } else {
                panic!("Unhandled byte marker: {:02x} {:02x} (i={}/{})",
                       vec[i],
                       vec[i + 1],
                       i,
                       vec.len());
            }
        }
        Ok(image)
    }

    pub fn width(&self) -> usize {
        self.dimensions.0 as usize
    }

    pub fn height(&self) -> usize {
        self.dimensions.1 as usize
    }

    pub fn image_data(&self) -> Option<&Vec<(u8, u8, u8)>> {
        self.image_data.as_ref()
    }
}
