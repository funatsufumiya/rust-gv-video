// decode_bc2_alpha, decode_bc2_block: dual-licensed under Apache 2.0 and MIT, by @autergame
// see https://github.com/autergame/texture2ddecoder/commit/6a5e8eaa8a146a0ed117fc245ebfbefa06abe8a5
//
// other code from texture2ddecoder, dual-licensed under Apache 2.0 and MIT, by @UniversalGameExtraction
// see https://github.com/UniversalGameExtraction/texture2ddecoder

use texture2ddecoder::decode_bc1_block;

#[inline]
pub fn decode_bc2_alpha(data: &[u8], outbuf: &mut [u32], channel: usize) {
    let channel_shift = channel * 8;
    let channel_mask = 0xFFFFFFFF ^ (0xFF << channel_shift);
    (0..16).for_each(|i| {
        let bit_i = i * 4;
        let by_i = bit_i >> 3;
        let av = 0xf & (data[by_i] >> (bit_i & 7));
        let av = (av << 4) | av;
        outbuf[i] = (outbuf[i] & channel_mask) | (av as u32) << channel_shift;
    });
}

#[inline]
pub fn decode_bc2_block(data: &[u8], outbuf: &mut [u32]) {
    decode_bc1_block(&data[8..], outbuf);
    decode_bc2_alpha(data, outbuf, 3);
}

#[inline]
pub fn copy_block_buffer(
    bx: usize,
    by: usize,
    w: usize,
    h: usize,
    bw: usize,
    bh: usize,
    buffer: &[u32],
    image: &mut [u32],
) {
    let x: usize = bw * bx;
    let copy_width: usize = if bw * (bx + 1) > w { w - bw * bx } else { bw };

    let y_0 = by * bh;
    let copy_height: usize = if bh * (by + 1) > h { h - y_0 } else { bh };
    let mut buffer_offset = 0;

    for y in y_0..y_0 + copy_height {
        let image_offset = y * w + x;
        image[image_offset..image_offset + copy_width]
            .copy_from_slice(&buffer[buffer_offset..buffer_offset + copy_width]);

        buffer_offset += bw;
    }
}

#[inline]
pub const fn color(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_le_bytes([b, g, r, a])
}

// macro to generate generic block decoder functions
macro_rules! block_decoder{
    ($name: expr, $block_width: expr, $block_height: expr, $raw_block_size: expr, $block_decode_func: expr) => {
        paste::item! {
            #[doc = "Decodes a " $name " encoded texture into an image"]
            pub fn [<decode_ $name>](data: &[u8], width: usize, height: usize, image: &mut [u32]) -> Result<(), &'static str> {
                const BLOCK_WIDTH: usize = $block_width;
                const BLOCK_HEIGHT: usize = $block_height;
                const BLOCK_SIZE: usize = BLOCK_WIDTH * BLOCK_HEIGHT;
                let num_blocks_x: usize = (width + BLOCK_WIDTH - 1) / BLOCK_WIDTH;
                let num_blocks_y: usize = (height + BLOCK_WIDTH - 1) / BLOCK_HEIGHT;
                let mut buffer: [u32; BLOCK_SIZE] = [color(0,0,0,255); BLOCK_SIZE];

                if data.len() < num_blocks_x * num_blocks_y * $raw_block_size {
                    return Err("Not enough data to decode image!");
                }

                if image.len() < width * height {
                    return Err("Image buffer is too small!");
                }

                let mut data_offset = 0;
                (0..num_blocks_y).for_each(|by| {
                    (0..num_blocks_x).for_each(|bx| {
                        $block_decode_func(&data[data_offset..], &mut buffer);
                        copy_block_buffer(
                            bx,
                            by,
                            width,
                            height,
                            BLOCK_WIDTH,
                            BLOCK_HEIGHT,
                            &buffer,
                            image,
                        );
                        data_offset += $raw_block_size;
                    });
                });
                Ok(())
            }

        }
    };
}

block_decoder!("bc2", 4, 4, 16, decode_bc2_block);