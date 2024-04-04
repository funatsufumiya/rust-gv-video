# GV Video Decoder for Rust

Port of GV video (Extreme Gpu Friendly Video Format) https://github.com/Ushio/ofxExtremeGpuVideo#binary-file-format-gv decoder for Rust.

- This crate provides both `BC1(DXT1)/BC2(DXT3)/BC3(DXT5)/BC7` decoder and LZ4 decompressor, but recommended **NOT** to use `BC1/BC2/BC3/BC7` decoder because it's CPU processing (slow).
  - you can get LZ4 decompressed (not BC decoded) frame with `read_frame_compressed(index)` and `read_frame_compressed_at(time)` methods. (fastest way for GPU texture upload)
  - you can get BC decoded and LZ4 decompressed frame with `read_frame(index)` and `read_frame_at(time)` methods. (easy for BRGA texture checking and CPU processing)
- This crate **NOT** provides movie player function. Please use like [bevy_movie_player](https://github.com/funatsufumiya/bevy_movie_player) crate for it (as an alternative of [ofxExtremeGpuVideo](https://github.com/Ushio/ofxExtremeGpuVideo) for openframeworks).
- This crate **NOT** provides encoder for now (but planning to provide it in the future.) Currently, you can use [ofxExtremeGpuVideo](https://github.com/Ushio/ofxExtremeGpuVideo) tools for encoding.

## binary file format (gv)

```text
0: uint32_t width
4: uint32_t height
8: uint32_t frame count
12: float fps
16: uint32_t format (DXT1 = 1, DXT3 = 3, DXT5 = 5, BC7 = 7)
20: uint32_t frame bytes
24: raw frame storage (lz4 compressed)
eof - (frame count) * 16: [(uint64_t, uint64_t)..<frame count] (address, size) of lz4, address is zero based from file head
```
