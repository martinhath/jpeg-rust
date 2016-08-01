
use jpeg::jfif::{FrameHeader, ScanHeader};

/// Struct to hold state of JPEG decoding.
/// Instantiate it, and pass in AC/DC tables, quantization
/// tables, sampling factors, data, etc. as it is available,
/// or updated.
///
/// Call `JPEGDecoder::decode()` to start reading from `data`.
pub struct JPEGDecoder<'a> {
    /// Encoded image data
    data: &'a [u8],

    /// Fields specific for each component.
    component_fields: Vec<JPEGDecoderComponentFields>,
}

impl<'a> JPEGDecoder<'a> {
    pub fn new(data: &'a [u8]) -> JPEGDecoder {
        JPEGDecoder {
            data: data,
            component_fields: Vec::new(),
        }
    }

    pub fn frame_header(mut self, frame_header: &'a FrameHeader) -> JPEGDecoder {
        for frame_component in &frame_header.frame_components {
            // Update horiz/vert sampling factor, and quant selector.
            let was_none = self.component_fields
                .iter_mut()
                .find(|cf| cf.component == frame_component.component_id)
                .as_mut()
                .map(|cf| {
                    cf.horizontal_sampling_factor = frame_component.horizontal_sampling_factor;
                    cf.vertical_sampling_factor = frame_component.vertical_sampling_factor;
                    cf.quantization_id = frame_component.quantization_selector;
                })
                .is_none();
            // Or insert a new element, if none was found.
            if was_none {
                self.component_fields.push({
                    JPEGDecoderComponentFields {
                        component: frame_component.component_id,
                        horizontal_sampling_factor: frame_component.horizontal_sampling_factor,
                        vertical_sampling_factor: frame_component.vertical_sampling_factor,
                        quantization_id: frame_component.quantization_selector,
                        dc_table_id: 0,
                        ac_table_id: 0,
                    }
                });
            }
        }
        self
    }

    pub fn scan_header(mut self, scan_header: &'a ScanHeader) -> JPEGDecoder {
        for scan_component in &scan_header.scan_components {
            // Update horiz/vert sampling factor, and quant selector.
            let was_none = self.component_fields
                .iter_mut()
                .find(|cf| cf.component == scan_component.component_id)
                .as_mut()
                .map(|cf| {
                    cf.ac_table_id = scan_component.ac_table_selector;
                    cf.dc_table_id = scan_component.dc_table_selector;
                })
                .is_none();
            // Or insert a new element, if none was found.
            if was_none {
                self.component_fields.push({
                    JPEGDecoderComponentFields {
                        component: scan_component.component_id,
                        horizontal_sampling_factor: 0,
                        vertical_sampling_factor: 0,
                        quantization_id: 0,
                        dc_table_id: scan_component.ac_table_selector,
                        ac_table_id: scan_component.dc_table_selector,
                    }
                });
            }
        }
        self
    }

    pub fn decode(&mut self) {}
}

#[derive(Debug, Clone)]
/// All component specific fields:
///
// TODO: Rather use Option<> on the fields, as they may not
//       be set?
struct JPEGDecoderComponentFields {
    /// Component ID
    component: u8,
    /// AC Huffman table id
    dc_table_id: u8,
    /// DC Huffman table id
    ac_table_id: u8,
    /// Quantization table id
    quantization_id: u8,
    /// Number of pixels for each sample in horizontal direction (?)
    horizontal_sampling_factor: u8,
    /// Number of pixels for each sample in horizontal direction (?)
    vertical_sampling_factor: u8,
}
