use rust_gv_video::gv_video::{get_rgba_from_frame, GVFormat, GVVideo, RGBAColor};
use std::{fs::File, io::BufReader};

fn main() {
        // const TEST_10PX_GV: &[u8; 474] = include_bytes!("../test_asset/test-10px.gv");

        // read file using bytereader and file
        let file = File::open("test_asset/test-10px.gv").unwrap();
        let mut reader = BufReader::new(file);

        let mut video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 10);
        assert_eq!(video.header.height, 10);
        assert_eq!(video.header.frame_count, 5);
        assert_eq!(video.header.fps, 1.0);
        assert_eq!(video.header.format, GVFormat::DXT1);
        assert_eq!(video.header.frame_bytes, 72);
        assert_eq!(video.get_duration(), std::time::Duration::from_secs_f32(5.0));

        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(3.5)).unwrap();
        assert_eq!(frame.len(), 10 * 10 * 4);

        // 4.99 sec
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(4.99)).unwrap();
        assert_eq!(frame.len(), 10 * 10 * 4);

        // 5.01 sec is out of range
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(5.01));
        assert!(frame.is_err());
        assert_eq!(frame.err(), Some("End of video"));

        // check x,y = 0,0 should be red
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(0.0)).unwrap();
        let rgba = get_rgba_from_frame(&frame, 0, 0, 10);
        assert_eq!(rgba, RGBAColor { r: 255, g: 0, b: 0, a: 255 });

}