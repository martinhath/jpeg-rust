#[derive(Debug)]
pub enum JFIFUnits {
    NoUnits,
    DotsPerInch,
    DotsPerCm,
}

#[derive(Debug)]
pub enum JFIFVersion {
    V1_01,
}

impl JFIFVersion {
    pub fn from_bytes(msb: u8, lsb: u8) -> Option<JFIFVersion> {
        match msb {
            1 => {
                match lsb {
                    1 => return Some(JFIFVersion::V1_01),
                    _ => {}
                }
            }
            _ => {}
        }
        return None;
    }
}

pub struct JFIFHeader {

}

impl JFIFHeader {
    pub fn parse(vec: &Vec<u8>) -> Option<JFIFHeader> {
        // you can identify a JFIF file by looking for the following sequence:
        //
        //      X'FF', SOI, X'FF', APP0, <2 bytes to be skipped>, "JFIF", X'00'.
        if vec.len() < 11 {
            return None;
        }
        let SOI = 0xd8;
        let APP0 = 0xe0;
        if vec[0] != 0xff || vec[1] != SOI || vec[2] != 0xff || vec[3] != APP0 ||
           vec[6] != 'J' as u8 || vec[7] != 'F' as u8 || vec[8] != 'I' as u8 ||
           vec[9] != 'F' as u8 || vec[10] != 0x00 {
            return None;
        }
        let version = JFIFVersion::from_bytes(vec[11], vec[12]);
        println!("version: {:?}", version);

        None
    }
}
