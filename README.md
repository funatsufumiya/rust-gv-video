# GV Video Decoder for Rust

Port of GV video (Extreme Gpu Friendly Video Format) https://github.com/Ushio/ofxExtremeGpuVideo decoder to Rust.

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