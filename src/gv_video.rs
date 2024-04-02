// Extreme Gpu Friendly Video Format
//
// binary file format:
// 
// 0: uint32_t width
// 4: uint32_t height
// 8: uint32_t frame count
// 12: float fps
// 16: uint32_t format (DXT1 = 1, DXT3 = 3, DXT5 = 5, BC7 = 7)
// 20: uint32_t frame bytes
// 24: raw frame storage (lz4 compressed)
// eof - (frame count) * 16: [(uint64_t, uint64_t)..<frame count] (address, size) of lz4, address is zero based from file head
//

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, PartialEq, Eq)]
pub enum Format {
    DXT1 = 1,
    DXT3 = 3,
    DXT5 = 5,
    BC7 = 7,
}

// const HEADER_SIZE: usize = 24;

pub struct Header {
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub fps: f32,
    pub format: Format,
    pub frame_bytes: u32,
}

pub struct Decoder<Reader> {
    reader: Reader,
}

pub struct GVVideo<Reader> {
    pub header: Header,
    pub decoder: Decoder<Reader>,
}

pub fn read_header<Reader>(reader: &mut Reader) -> Header where Reader: std::io::Read {
    let width = reader.read_u32::<LittleEndian>().unwrap();
    let height = reader.read_u32::<LittleEndian>().unwrap();
    let frame_count = reader.read_u32::<LittleEndian>().unwrap();
    let fps = reader.read_f32::<LittleEndian>().unwrap();
    let format = reader.read_u32::<LittleEndian>().unwrap();
    let frame_bytes = reader.read_u32::<LittleEndian>().unwrap();
    Header {
        width,
        height,
        frame_count,
        fps,
        format: match format {
            1 => Format::DXT1,
            3 => Format::DXT3,
            5 => Format::DXT5,
            7 => Format::BC7,
            _ => panic!("Unknown format"),
        },
        frame_bytes,
    }
}

impl<Reader: std::io::Read> GVVideo<Reader> {
    pub fn load(reader: Reader) -> GVVideo<Reader> {
        let mut reader = reader;
        let header = read_header(&mut reader);
        GVVideo {
            header,
            decoder: Decoder { reader },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn header_read() {
        let data = vec![
            0x02, 0x00, 0x00, 0x00, // width
            0x02, 0x00, 0x00, 0x00, // height
            0x02, 0x00, 0x00, 0x00, // frame count
            0x00, 0x00, 0x80, 0x3F, // fps
            0x01, 0x00, 0x00, 0x00, // format
            0x04, 0x00, 0x00, 0x00, // frame bytes
        ];
        let mut reader = Cursor::new(data);
        let video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 2);
        assert_eq!(video.header.height, 2);
        assert_eq!(video.header.frame_count, 2);
        assert_eq!(video.header.fps, 1.0);
        assert_eq!(video.header.format, Format::DXT1);
        assert_eq!(video.header.frame_bytes, 4);
    }

    // #[test]
    // fn header_read_with_file() {
    //     let data = include_bytes!("../test.gv");
    //     let mut reader = Cursor::new(data);
    //     let video = GVVideo::load(&mut reader);
    //     assert_eq!(video.header.width, 1280);
    //     assert_eq!(video.header.height, 720);
    //     assert_eq!(video.header.frame_count, 30);
    //     assert_eq!(video.header.fps, 30.0);
    //     assert_eq!(video.header.format, Format::DXT1);
    //     assert_eq!(video.header.frame_bytes, 460800);
    // }
}