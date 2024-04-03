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
use texture2ddecoder;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

pub struct GVVideo<Reader> {
    pub header: Header,
    pub reader: Reader,
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

impl<Reader: std::io::Read + std::io::Seek> GVVideo<Reader> {
    pub fn load(reader: Reader) -> GVVideo<Reader> {
        let mut reader = reader;
        let header = read_header(&mut reader);
        GVVideo {
            header,
            reader,
        }
    }

    fn decode_dxt(&mut self, data: Vec<u8>) -> Vec<u32> {
        let width = self.header.width as usize;
        let height = self.header.height as usize;
        let format = self.header.format;
        let uncompressed_size = (width * height * 4) as usize;
        let lz4_decoded_data = lz4_flex::block::decompress(&data, uncompressed_size).unwrap();
        let mut result = vec![0; uncompressed_size];

        match format {
            Format::DXT1 => {
                let res = texture2ddecoder::decode_bc1(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT1: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            Format::DXT3 => {
                let res = texture2ddecoder::decode_bc2(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT3: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            Format::DXT5 => {
                let res = texture2ddecoder::decode_bc3(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT5: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            Format::BC7 => {
                let res = texture2ddecoder::decode_bc7(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding BC7: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
        }
    }

    pub fn read_frame(&mut self, frame_id: u32) -> Result<Vec<u32>, &'static str> {
        if frame_id >= self.header.frame_count {
            return Err("End of video");
        }

        // f.seek(-frame_count * 16 + i * 16, os.SEEK_END)

        println!("frame_id: {}", frame_id);
        println!("debug: {}", -((self.header.frame_count * 16) as i64) + (frame_id as i64 * 16));
        
        self.reader.seek(std::io::SeekFrom::End(
            -((self.header.frame_count * 16) as i64) + (frame_id as i64 * 16))
        ).unwrap();

        let address = self.reader.read_u64::<LittleEndian>().unwrap_or(0);
        let size = self.reader.read_u64::<LittleEndian>().unwrap_or(0) as usize;
        if address == 0 || size == 0 {
            return Err("Error reading frame address or size");
        }else{
            println!("address: {}", address);
            println!("size: {}", size);
            
            let mut data = vec![0; size];
            // let mut data = vec![0; (size * 4) as usize];

            if let Err(_) = self.reader.seek(std::io::SeekFrom::Start(address)) {
                return Err("Error seeking frame data");
            }
            if let Err(_) = self.reader.read_exact(&mut data) {
                return Err("Error reading frame data");
            }

            Ok(self.decode_dxt(data))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TEST_GV: &[u8; 1864] = include_bytes!("../test_asset/test.gv");

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

    #[test]
    fn header_read_with_file() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 1280);
        assert_eq!(video.header.height, 720);
        assert_eq!(video.header.frame_count, 1);
        assert_eq!(video.header.fps, 30.0);
        assert_eq!(video.header.format, Format::DXT1);
        assert_eq!(video.header.frame_bytes, 460800);
    }

    #[test]
    fn read_first_frame() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame(0).unwrap();
        assert_eq!(frame.len(), 1280 * 720 * 4);
    }

    #[test]
    fn read_second_frame_then_error() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame(1);
        assert!(frame.is_err());
        assert_eq!(frame.err(), Some("End of video"));
    }
}