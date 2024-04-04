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

use std::{fs::File, io::{BufReader, Read, Seek}, mem};

use byteorder::{LittleEndian, ReadBytesExt};
use texture2ddecoder;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GVFormat {
    DXT1 = 1,
    DXT3 = 3,
    DXT5 = 5,
    BC7 = 7,
}

// const HEADER_SIZE: usize = 24;

#[derive(Debug)]
pub struct GVHeader {
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub fps: f32,
    pub format: GVFormat,
    pub frame_bytes: u32,
}

#[derive(Debug)]
pub struct GVVideo<Reader: Read + Seek> {
    pub header: GVHeader,
    pub reader: Reader,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RGBAColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RGBColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn get_rgba(color: u32) -> RGBAColor {
    RGBAColor {
        r: (color >> 16) as u8,
        g: (color >> 8) as u8,
        b: color as u8,
        a: (color >> 24) as u8,
    }
}

pub fn get_rgb(color: u32) -> RGBColor {
    RGBColor {
        r: (color >> 16) as u8,
        g: (color >> 8) as u8,
        b: color as u8,
    }
}

pub fn get_alpha(color: u32) -> u8 {
    (color >> 24) as u8
}

pub fn get_rgba_from_frame(frame: &Vec<u32>, x: usize, y: usize, width: usize) -> RGBAColor {
    get_rgba(frame[x + y * width])
}

pub fn get_rgb_from_frame(frame: &Vec<u32>, x: usize, y: usize, width: usize) -> RGBColor {
    get_rgb(frame[x + y * width])
}

pub fn get_alpha_from_frame(frame: &Vec<u32>, x: usize, y: usize, width: usize) -> u8 {
    get_alpha(frame[x + y * width])
}

/// Vec<u32>'s u32 is showing ARGB as little endian, this convert it to RGBA u8
/// ex: [0xFFAABBCC, 0xFFDDEE88] -> [0xAA, 0xBB, 0xCC, 0xFF, 0xDD, 0xEE, 0x88, 0xFF]
pub fn get_rgba_vec_from_frame(frame: &Vec<u32>) -> Vec<u8> {
    // FIXME: more efficient way?
    let mut result = Vec::with_capacity(frame.len() * 4);
    for color in frame {
        result.push((color >> 16) as u8);
        result.push((color >> 8) as u8);
        result.push((color >> 0) as u8);
        result.push((color >> 24) as u8);
    }
    result
}

/// Vec<u32>'s u32 is showing ARGB as little endian, this convert it to RGB u8
/// ex: [0xFFAABBCC, 0xFFDDEE88] -> [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x88]
pub fn get_rgb_vec_from_frame(frame: &Vec<u32>) -> Vec<u8> {
    // FIXME: more efficient way?
    let mut result = Vec::with_capacity(frame.len() * 3);
    for color in frame {
        result.push((color >> 16) as u8);
        result.push((color >> 8) as u8);
        result.push((color >> 0) as u8);
    }
    result
}

// faster but unsafe
pub fn to_vec_u8_unsafe(mut frame: Vec<u32>) -> Vec<u8> {
    // https://stackoverflow.com/questions/49690459/converting-a-vecu32-to-vecu8-in-place-and-with-minimal-overhead
    let vec8 = unsafe {
        let ratio = mem::size_of::<u32>() / mem::size_of::<u8>();

        let length = frame.len() * ratio;
        let capacity = frame.capacity() * ratio;
        let ptr = frame.as_mut_ptr() as *mut u8;

        // Don't run the destructor for vec32
        mem::forget(frame);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    };

    vec8
}

pub fn read_header<Reader>(reader: &mut Reader) -> GVHeader where Reader: std::io::Read {
    let width = reader.read_u32::<LittleEndian>().unwrap();
    let height = reader.read_u32::<LittleEndian>().unwrap();
    let frame_count = reader.read_u32::<LittleEndian>().unwrap();
    let fps = reader.read_f32::<LittleEndian>().unwrap();
    let format = reader.read_u32::<LittleEndian>().unwrap();
    let frame_bytes = reader.read_u32::<LittleEndian>().unwrap();
    GVHeader {
        width,
        height,
        frame_count,
        fps,
        format: match format {
            1 => GVFormat::DXT1,
            3 => GVFormat::DXT3,
            5 => GVFormat::DXT5,
            7 => GVFormat::BC7,
            _ => panic!("Unknown format"),
        },
        frame_bytes,
    }
}

impl<Reader: Read + Seek> GVVideo<Reader> {
    pub fn load(mut reader: Reader) -> GVVideo<Reader> {
        let header = read_header(&mut reader);
        GVVideo {
            header,
            reader,
        }
    }

    pub fn load_from_file(file_path: &str) -> GVVideo<BufReader<File>> {
        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);
        GVVideo::load(reader)
    }

    fn decode_dxt(&mut self, data: Vec<u8>) -> Vec<u32> {
        let width = self.header.width as usize;
        let height = self.header.height as usize;
        let format = self.header.format;
        let uncompressed_size_u8 = (width * height * 4) as usize;
        let uncompressed_size_u32 = (width * height) as usize;
        let lz4_decoded_data = lz4_flex::block::decompress(&data, uncompressed_size_u8).unwrap();
        let mut result = vec![0; uncompressed_size_u32];

        match format {
            GVFormat::DXT1 => {
                let res = texture2ddecoder::decode_bc1(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT1: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            GVFormat::DXT3 => {
                let res = texture2ddecoder::decode_bc2(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT3: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            GVFormat::DXT5 => {
                let res = texture2ddecoder::decode_bc3(&lz4_decoded_data, width, height, &mut result);
                if res.is_err() {
                    panic!("Error decoding DXT5: {:?}", res.err().unwrap());
                }else{
                    result
                }
            }
            GVFormat::BC7 => {
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

        // println!("frame_id: {}", frame_id);
        // println!("debug: {}", -((self.header.frame_count * 16) as i64) + (frame_id as i64 * 16));
        
        self.reader.seek(std::io::SeekFrom::End(
            -((self.header.frame_count * 16) as i64) + (frame_id as i64 * 16))
        ).unwrap();

        let address = self.reader.read_u64::<LittleEndian>().unwrap_or(0);
        let size = self.reader.read_u64::<LittleEndian>().unwrap_or(0) as usize;
        if address == 0 || size == 0 {
            return Err("Error reading frame address or size");
        }else{
            // println!("address: {}", address);
            // println!("size: {}", size);
            
            let mut data = vec![0; size];

            if let Err(_) = self.reader.seek(std::io::SeekFrom::Start(address)) {
                return Err("Error seeking frame data");
            }
            if let Err(_) = self.reader.read_exact(&mut data) {
                return Err("Error reading frame data");
            }

            Ok(self.decode_dxt(data))
        }
    }

    pub fn read_frame_at(&mut self, duration: std::time::Duration) -> Result<Vec<u32>, &'static str> {
        let frame_id = (self.header.fps * duration.as_secs_f32()) as u32;
        self.read_frame(frame_id)
    }

    pub fn get_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.header.frame_count as f32 / self.header.fps)
    }

    pub fn get_width(&self) -> u32 {
        self.header.width
    }

    pub fn get_height(&self) -> u32 {
        self.header.height
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.header.width, self.header.height)
    }

    pub fn get_frame_count(&self) -> u32 {
        self.header.frame_count
    }

    pub fn get_fps(&self) -> f32 {
        self.header.fps
    }

    pub fn get_format(&self) -> GVFormat {
        self.header.format
    }

    pub fn get_frame_bytes(&self) -> u32 {
        self.header.frame_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // SMPTE BAR
    const TEST_GV: &[u8; 1547] = include_bytes!("../test_asset/test.gv");
    // SMPTE BAR with alpha gradient
    const TEST_ALPHA_GV: &[u8; 4857] = include_bytes!("../test_asset/test-alpha.gv");
    // 10px 5sec 1fps
    const TEST_10PX_GV: &[u8; 474] = include_bytes!("../test_asset/test-10px.gv");

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
        assert_eq!(video.header.format, GVFormat::DXT1);
        assert_eq!(video.header.frame_bytes, 4);
    }

    #[test]
    fn header_read_with_file() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 640);
        assert_eq!(video.header.height, 360);
        assert_eq!(video.header.frame_count, 1);
        assert_eq!(video.header.fps, 30.0);
        assert_eq!(video.header.format, GVFormat::DXT1);
        assert_eq!(video.header.frame_bytes, 115200);
    }

    #[test]
    fn read_first_frame() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame(0).unwrap();
        assert_eq!(frame.len(), 640 * 360);
    }

    #[test]
    fn read_rgba() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame(0).unwrap();
        // rgba: 189, 190, 189, 255
        assert_eq!(frame[0], 0xFFBDBEBD);
        // rgba: 192, 190, 0, 255
        assert_eq!(frame[130], 0xFFC0BE00);
        // rgba: 0, 188, 0, 255
        assert_eq!(frame[320], 0xFF00BC00);
        // rgba: 0, 0, 192, 255
        assert_eq!(frame[595], 0xFF0000C0);

        // x, y = 160, 300 | white
        assert_eq!(frame[160 + 300 * 640], 0xFFFFFFFF);

        // x, y = 300, 300 | rgba: 62, 0, 118, 255
        assert_eq!(frame[300 + 300 * 640], 0xFF3E0076);

        assert_eq!(get_rgba(frame[0]), RGBAColor { r: 189, g: 190, b: 189, a: 255 });
        assert_eq!(get_rgb(frame[0]), RGBColor { r: 189, g: 190, b: 189 });
        assert_eq!(get_alpha(frame[0]), 0xFF);

        assert_eq!(get_rgba(frame[130]), RGBAColor { r: 192, g: 190, b: 0, a: 255 });
        assert_eq!(get_rgb(frame[130]), RGBColor { r: 192, g: 190, b: 0 });
        assert_eq!(get_alpha(frame[130]), 0xFF);

        assert_eq!(get_rgba(frame[320]), RGBAColor { r: 0, g: 188, b: 0, a: 255 });
        assert_eq!(get_rgb(frame[320]), RGBColor { r: 0, g: 188, b: 0 });
        assert_eq!(get_alpha(frame[320]), 0xFF);

        assert_eq!(get_rgba(frame[595]), RGBAColor { r: 0, g: 0, b: 192, a: 255 });
        assert_eq!(get_rgb(frame[595]), RGBColor { r: 0, g: 0, b: 192 });
        assert_eq!(get_alpha(frame[595]), 0xFF);

        assert_eq!(get_rgba(frame[160 + 300 * 640]), RGBAColor { r: 255, g: 255, b: 255, a: 255 });

        assert_eq!(get_rgba(frame[300 + 300 * 640]), RGBAColor { r: 62, g: 0, b: 118, a: 255 });
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

    #[test]
    fn check_alpha() {
        let data = TEST_ALPHA_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame(0).unwrap();

        // rgba: 189, 190, 189, 255
        assert_eq!(frame[0], 0xFFBDBEBD);
        // rgba: 192, 190, 0, 228
        assert_eq!(frame[130], 0xE4C0BE00);
        // rgba: 0, 188, 0, 130
        assert_eq!(frame[320], 0x8200BC00);
        // rgba: 0, 0, 192, 0
        assert_eq!(frame[595], 0x000000C0);

        // x, y = 160, 300 | rgba: 255, 255, 255, 212
        assert_eq!(frame[160 + 300 * 640], 0xD4FFFFFF);

        // x, y = 300, 300 | rgba: 62, 0, 118, 140
        assert_eq!(frame[300 + 300 * 640], 0x8C3E0076);

        assert_eq!(get_rgba(frame[0]), RGBAColor { r: 189, g: 190, b: 189, a: 255 });
        assert_eq!(get_rgb(frame[0]), RGBColor { r: 189, g: 190, b: 189 });
        assert_eq!(get_alpha(frame[0]), 0xFF);

        assert_eq!(get_rgba(frame[130]), RGBAColor { r: 192, g: 190, b: 0, a: 228 });
        assert_eq!(get_rgb(frame[130]), RGBColor { r: 192, g: 190, b: 0 });
        assert_eq!(get_alpha(frame[130]), 0xE4);

        assert_eq!(get_rgba(frame[320]), RGBAColor { r: 0, g: 188, b: 0, a: 130 });
        assert_eq!(get_rgb(frame[320]), RGBColor { r: 0, g: 188, b: 0 });
        assert_eq!(get_alpha(frame[320]), 0x82);

        assert_eq!(get_rgba(frame[595]), RGBAColor { r: 0, g: 0, b: 192, a: 0 });
        assert_eq!(get_rgb(frame[595]), RGBColor { r: 0, g: 0, b: 192 });
        assert_eq!(get_alpha(frame[595]), 0x00);

        assert_eq!(get_rgba(frame[160 + 300 * 640]), RGBAColor { r: 255, g: 255, b: 255, a: 212 });

        assert_eq!(get_rgba(frame[300 + 300 * 640]), RGBAColor { r: 62, g: 0, b: 118, a: 140 });
    }

    #[test]
    fn read_frame_at() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(0.0)).unwrap();
        assert_eq!(frame.len(), 640 * 360);
    }

    #[test]
    fn read_frame_at_with_error() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(1.0));
        assert!(frame.is_err());
        assert_eq!(frame.err(), Some("End of video"));
    }

    #[test]
    fn check_duration1() {
        let data = TEST_GV;
        let mut reader = Cursor::new(data);
        let video = GVVideo::load(&mut reader);
        assert_eq!(video.get_duration(), std::time::Duration::from_secs_f32(1.0 / 30.0));
    }

    #[test]
    fn check_duration2() {
        let data = TEST_10PX_GV;
        let mut reader = Cursor::new(data);
        let video = GVVideo::load(&mut reader);
        assert_eq!(video.get_duration(), std::time::Duration::from_secs_f32(5.0));
    }

    #[test]
    fn read_frame_at_3_5() {
        let data = TEST_10PX_GV;
        let mut reader = Cursor::new(data);
        let mut video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 10);
        assert_eq!(video.header.height, 10);
        assert_eq!(video.header.frame_count, 5);
        assert_eq!(video.header.fps, 1.0);
        assert_eq!(video.header.format, GVFormat::DXT1);
        assert_eq!(video.header.frame_bytes, 72);
        assert_eq!(video.get_duration(), std::time::Duration::from_secs_f32(5.0));

        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(3.5)).unwrap();
        assert_eq!(frame.len(), 10 * 10);

        // 4.99 sec
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(4.99)).unwrap();
        assert_eq!(frame.len(), 10 * 10);

        // 5.01 sec is out of range
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(5.01));
        assert!(frame.is_err());
        assert_eq!(frame.err(), Some("End of video"));
    }

    #[test]
    fn rgba_vec() {
        let test_vec = vec![0xFFAABBCC, 0xFFDDEE88];
        let result = get_rgba_vec_from_frame(&test_vec);
        assert_eq!(result, vec![0xAA, 0xBB, 0xCC, 0xFF, 0xDD, 0xEE, 0x88, 0xFF]);
    }
}