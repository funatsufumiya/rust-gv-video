use gv_video::{get_rgba_from_frame, get_rgba_vec_from_frame, GVFormat, GVVideo, RGBAColor};
use std::{fs::File, io::BufReader};

fn main() {
        let file = File::open("test_asset/test-10px.gv").unwrap();
        let mut reader = BufReader::new(file);

        let (w, h) = (10, 10);

        let mut video = GVVideo::load(&mut reader);
        assert_eq!(video.header.width, 10);
        assert_eq!(video.header.height, 10);
        assert_eq!(video.header.frame_count, 5);
        assert_eq!(video.header.fps, 1.0);
        assert_eq!(video.header.format, GVFormat::DXT1);
        assert_eq!(video.header.frame_bytes, 72);
        assert_eq!(video.get_duration(), std::time::Duration::from_secs_f32(5.0));

        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(3.5)).unwrap();
        assert_eq!(frame.len(), w * h);
        // type of frame is Vec<u32>
        assert_eq!(frame[0], 0xFFFF0000); // x,y=0,0: red (0xAARRGGBB)
        assert_eq!(frame[6], 0xFF0000FF); // x,y=6,0: blue (0xAARRGGBB)
        assert_eq!(frame[0 + w*6], 0xFF00FF00); // x,y=0,6: green (0xAARRGGBB)
        assert_eq!(frame[6 + w*6], 0xFFE7FF00); // x,y=6,6: yellow (0xAARRGGBB)

        // 4.99 sec
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(4.99)).unwrap();
        assert_eq!(frame.len(), w * h);

        // 5.01 sec is out of range
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(5.01));
        assert!(frame.is_err());
        assert_eq!(frame.err(), Some("End of video"));

        // you can also read frame by index
        let frame = video.read_frame(0).unwrap();
        assert_eq!(frame.len(), w * h);

        // check x,y = 0,0 should be red
        let frame = video.read_frame_at(std::time::Duration::from_secs_f32(0.0)).unwrap();
        let rgba = get_rgba_from_frame(&frame, 0, 0, w);
        assert_eq!(rgba, RGBAColor { r: 255, g: 0, b: 0, a: 255 });

        // you can convert frame to Vec<u8> ( [R,G,B,A,R,G,B,A,...] )
        let frame_u8 = get_rgba_vec_from_frame(&frame);
        assert_eq!(frame_u8.len(), w * h * 4);
        assert_eq!(frame_u8[0], 255); // R
        assert_eq!(frame_u8[1], 0); // G
        assert_eq!(frame_u8[2], 0); // B
        assert_eq!(frame_u8[3], 255); // A

        println!("All tests passed");
}