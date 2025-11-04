use hoteldusk_tools::util::{Color, ReadExt};
use std::{
    error::Error,
    io::{Cursor, Read, Seek},
    path::Path,
};

const FRAME_DURATION_MS: i32 = 150;
const MTC_WIDTH: usize = 17;
const MTC_HEIGHT: usize = 33;
const MTC_SUFFIX: &str = "m_.mtc";

fn main() -> Result<(), Box<dyn Error>> {
    let anm_files = std::env::args_os()
        .skip(1)
        .filter(|arg| {
            Path::new(&arg)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("anm"))
        })
        .collect::<Vec<_>>();

    if anm_files.is_empty() {
        println!("Usage: anm2webp anm_file(s) ...");
        return Ok(());
    }

    for file in &anm_files {
        let data = std::fs::read(file)?;
        let output = Path::new(file).with_extension("webp");

        let mut reader = Cursor::new(&data);
        let _unknown: u32 = reader.read_le()?;
        let frame_count: u32 = reader.read_le()?;
        let _some_len: u32 = reader.read_le()?;
        let default_color_index: u32 = reader.read_le()?;
        let width: u16 = reader.read_le()?;
        let height: u16 = reader.read_le()?;
        let _frame: u16 = reader.read_le()?;
        let _frame: u16 = reader.read_le()?;
        let _frame: u16 = reader.read_le()?;

        let mut frames: Vec<Vec<u8>> = Vec::with_capacity(frame_count as usize);
        reader.set_position(32);
        for _ in 0..frame_count {
            let pos = reader.read_le::<u32>()? as usize;
            let len = reader.read_le::<u32>()? as usize;
            reader.seek_relative(8)?; // skip unknown
            let mut frame_data = &data[pos..][..len];
            let zero: u32 = frame_data.read_le()?;
            let compressed_len: u32 = frame_data.read_le()?;
            let palette_len: u32 = frame_data.read_le()?;
            let palette_count = (palette_len / 2) as usize;
            let _some_pos: u32 = frame_data.read_le()?;
            let (compressed, mut palette_data) = frame_data.split_at(compressed_len as usize);

            assert!(zero == 0);

            let mut buf = [0; 2];
            let mut palette = Vec::with_capacity(palette_count);
            for _ in 0..palette_count {
                palette_data.read_exact(&mut buf)?;
                let color = Color::from_rgb555(buf);
                palette.push(color);
            }

            if let Some(frame) = decompress_frame(
                compressed,
                width,
                height,
                frames.last(),
                &palette,
                default_color_index as usize,
            ) {
                frames.push(frame);
            } else {
                continue;
            };
        }

        write_webp(&output, &frames, width as u32, height as u32);

        if let Some(overlay_frames) = get_overlay_frames(file) {
            apply_overlay(
                &mut frames,
                &overlay_frames,
                width as usize,
                height as usize,
            );

            let mut filename = output.file_stem().unwrap().to_os_string();
            filename.push(".mtc.webp");
            write_webp(filename.as_ref(), &frames, width as u32, height as u32);
        };
    }

    Ok(())
}

fn write_webp(output: &Path, frames: &[Vec<u8>], width: u32, height: u32) {
    let mut config = webp::WebPConfig::new().unwrap();
    config.lossless = 1;
    config.alpha_filtering = 0;
    config.alpha_compression = 0;
    config.quality = 100.0;
    config.filter_sharpness = 0; // off
    config.filter_strength = 0; // off
    config.autofilter = 0;
    config.preprocessing = 0; // none

    let mut encoder = webp::AnimEncoder::new(width as u32, height as u32, &config);
    encoder.set_bgcolor([255; 4]);
    encoder.set_loop_count(0); // infinite loop.

    for (i, frame) in frames.iter().enumerate() {
        let anim_frame = webp::AnimFrame::from_rgba(
            frame,
            width as u32,
            height as u32,
            i as i32 * FRAME_DURATION_MS,
        );
        encoder.add_frame(anim_frame);
    }

    let webp = encoder.encode();
    if let Err(err) = std::fs::write(output, &*webp) {
        eprintln!("{err}")
    };
}

fn decompress_frame(
    mut input: &[u8],
    width: u16,
    height: u16,
    maybe_prev_frame: Option<&Vec<u8>>,
    palette: &[Color],
    default_color_index: usize,
) -> Option<Vec<u8>> {
    let palette_count = palette.len();
    let default_color = palette[default_color_index % palette_count].as_ref();
    let mut frame = Vec::with_capacity(width as usize * height as usize * 4);

    // workaround for Br_bracelet_.anm
    // this one use a bit different decompression method
    let bracelet = input.len() == 49540;

    while let Ok(ctrl) = input.read_le::<u8>() {
        let f1 = (ctrl >> 7) & 1 == 1;
        let f2 = (ctrl >> 6) & 1 == 1;

        match (f1, f2) {
            (true, _) => {
                let count = (ctrl & 0x7F) as usize;

                match maybe_prev_frame {
                    Some(prev_frame) => {
                        let pos = frame.len();
                        let len = count * 4;
                        let colors = &prev_frame[pos..][..len];
                        frame.extend(colors);
                    }
                    None => frame.extend(default_color.iter().cycle().take(count * 4)),
                }
            }
            // workaround for bradley's bracelet image
            (false, _) if bracelet => {
                let count = (ctrl & 0x7F) as usize;

                for _ in 0..count {
                    let color_index = input.read_le::<u8>().ok()? as usize;
                    let color = palette[color_index % palette_count].as_ref();
                    frame.extend(color);
                }
            }
            (false, true) => {
                let count = (ctrl & 0x3F) as usize;

                let color_index = input.read_le::<u8>().ok()? as usize;
                let color = palette[color_index % palette_count].as_ref();
                frame.extend(color.iter().cycle().take(count * 4));
            }
            (false, false) => {
                let count = (ctrl & 0x3F) as usize;

                for _ in 0..count {
                    let color_index = input.read_le::<u8>().ok()? as usize;
                    let color = palette[color_index % palette_count].as_ref();
                    frame.extend(color);
                }
            }
        }
    }

    Some(frame)
}

fn get_overlay_frames(anm_file: impl AsRef<Path>) -> Option<Vec<Vec<u8>>> {
    let anm_file = anm_file.as_ref();
    let mut mtc_file = anm_file.file_stem()?.to_os_string();
    mtc_file.push(MTC_SUFFIX);
    let path = anm_file.with_file_name(mtc_file);
    let data = std::fs::read(path).ok()?;
    let mut reader = data.as_slice();
    let frame_count: u32 = reader.read_le().ok()?;
    let _unused: [u8; 28] = reader.read_bytes().ok()?;

    let mut frames = Vec::with_capacity(frame_count as usize);
    for _ in 0..frame_count {
        let mut frame = Vec::with_capacity(MTC_WIDTH * MTC_HEIGHT * 4);
        let mut buf = [0; 2];

        for _ in 0..MTC_WIDTH * MTC_HEIGHT {
            reader.read_exact(&mut buf).ok()?;
            frame.extend(Color::from_rgb555(buf).as_ref());
        }

        frames.push(frame);
    }

    Some(frames)
}

fn apply_overlay(frames: &mut [Vec<u8>], overlay_frames: &[Vec<u8>], w: usize, h: usize) {
    for (frame, overlay) in frames.iter_mut().zip(overlay_frames.iter()) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            // this address different orientation between frame and the overlay frame
            let x = i / w;
            let y = w - 1 - (i % w);
            // swap the width and the height argument to address the different orientation
            let overlay_color = get_overlay_color(overlay, x, y, h, w);
            let frame_color: [u8; 4] = pixel.try_into().unwrap();
            let multiplied = multiply_color(frame_color, overlay_color);
            pixel.copy_from_slice(multiplied.as_slice());
        }
    }
}

fn get_overlay_color(overlay_frame: &[u8], x: usize, y: usize, w: usize, h: usize) -> [u8; 4] {
    let x_scale = (MTC_WIDTH - 1) as f32 / (w - 1) as f32;
    let y_scale = (MTC_HEIGHT - 1) as f32 / (h - 1) as f32;

    let x = x as f32 * x_scale;
    let y = y as f32 * y_scale;

    let x0 = x.floor() as usize;
    let x1 = (x + 1.0).min(MTC_WIDTH as f32 - 1.0).floor() as usize;
    let y0 = y.floor() as usize;
    let y1 = (y + 1.0).min(MTC_HEIGHT as f32 - 1.0).floor() as usize;

    let get_color = |x, y| -> [u8; 4] {
        let pos = ((y * MTC_WIDTH) + x) * 4;
        overlay_frame[pos..][..4].try_into().unwrap()
    };

    let tl = get_color(x0, y0);
    let tr = get_color(x1, y0);
    let bl = get_color(x0, y1);
    let br = get_color(x1, y1);

    let wx = x.fract();
    let wy = y.fract();

    let lerp_top = lerp_color(tl, tr, wx);
    let lerp_bottom = lerp_color(bl, br, wx);
    lerp_color(lerp_top, lerp_bottom, wy)
}

fn lerp_color(color: [u8; 4], color1: [u8; 4], t: f32) -> [u8; 4] {
    [
        (color[0] as f32 * (1.0 - t) + color1[0] as f32 * t) as u8,
        (color[1] as f32 * (1.0 - t) + color1[1] as f32 * t) as u8,
        (color[2] as f32 * (1.0 - t) + color1[2] as f32 * t) as u8,
        (color[3] as f32 * (1.0 - t) + color1[3] as f32 * t) as u8,
    ]
}

fn multiply_color(color1: [u8; 4], color2: [u8; 4]) -> [u8; 4] {
    const MAX: u16 = 0xFF;
    [
        (u16::from(color1[0]) * u16::from(color2[0]) / MAX) as u8,
        (u16::from(color1[1]) * u16::from(color2[1]) / MAX) as u8,
        (u16::from(color1[2]) * u16::from(color2[2]) / MAX) as u8,
        (u16::from(color1[3]) * u16::from(color2[3]) / MAX) as u8,
    ]
}

// fn multiply_color(color1: [u8; 4], color2: [u8; 4]) -> [u8; 4] {
//     let max = f32::from(u8::MAX);
//     let color1 = [
//         f32::from(color1[0]) / max,
//         f32::from(color1[1]) / max,
//         f32::from(color1[2]) / max,
//         f32::from(color1[3]) / max,
//     ];
//     let color2 = [
//         f32::from(color2[0]) / max,
//         f32::from(color2[1]) / max,
//         f32::from(color2[2]) / max,
//         f32::from(color2[3]) / max,
//     ];

//     [
//         ((color1[0] * color2[0]) * max) as u8,
//         ((color1[1] * color2[1]) * max) as u8,
//         ((color1[2] * color2[2]) * max) as u8,
//         ((color1[3] * color2[3]) * max) as u8,
//     ]
// }
