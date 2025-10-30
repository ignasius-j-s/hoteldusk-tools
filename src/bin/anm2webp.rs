use hoteldusk_tools::util::{Color, ReadExt};
use std::{
    error::Error,
    io::{Cursor, Read, Seek},
    path::Path,
};

const FRAME_DURATION_MS: i32 = 150;

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

    let mut config = webp::WebPConfig::new().unwrap();
    config.lossless = 1;
    config.alpha_filtering = 0;
    config.alpha_compression = 0;
    config.quality = 100.0;
    config.filter_sharpness = 0; // off
    config.filter_strength = 0; // off
    config.autofilter = 0;
    config.preprocessing = 0; // none

    for file in &anm_files {
        let data = std::fs::read(file)?;
        let output = Path::new(file).with_extension("webp");

        let mut reader = Cursor::new(&data);
        let _unknown: u32 = reader.read_le()?;
        let frame_count: u32 = reader.read_le()?;
        let _some_len: u32 = reader.read_le()?;
        let _unknown: u32 = reader.read_le()?;
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
                let color = Color::from_bgr555(buf);
                palette.push(color);
            }

            let last_frame = frames.last();
            if let Some(frame) = decompress_frame(compressed, width, height, last_frame, &palette) {
                frames.push(frame);
            } else {
                continue;
            };
        }

        let mut encoder = webp::AnimEncoder::new(width as u32, height as u32, &config);
        encoder.set_bgcolor([255; 4]);
        // infinite loop? not documented.
        encoder.set_loop_count(0);

        for (i, frame) in frames.iter().enumerate() {
            let anim_fram = webp::AnimFrame::from_rgba(
                frame,
                width as u32,
                height as u32,
                i as i32 * FRAME_DURATION_MS,
            );
            encoder.add_frame(anim_fram);
        }

        let webp = encoder.encode();
        std::fs::write(output, &*webp).ok();
    }

    Ok(())
}

fn decompress_frame(
    mut input: &[u8],
    width: u16,
    height: u16,
    last_frame: Option<&Vec<u8>>,
    palette: &[Color],
) -> Option<Vec<u8>> {
    let palette_count = palette.len();
    let last_color = palette.last().unwrap().as_ref();
    let mut frame = Vec::with_capacity(width as usize * height as usize * 4);

    while let Ok(ctrl) = input.read_le::<u8>() {
        let f1 = (ctrl >> 7) & 1 == 1;
        let f2 = (ctrl >> 6) & 1 == 1;

        match (f1, f2) {
            (true, _) => {
                let count = (ctrl & 0x7F) as usize;
                assert!(count != 0);

                match last_frame {
                    Some(last_frame) => {
                        let pos = frame.len();
                        let len = count * 4;
                        let colors = &last_frame[pos..][..len];
                        frame.extend(colors);
                    }
                    None => frame.extend(last_color.iter().cycle().take(count * 4)),
                }
            }
            (false, true) => {
                let count = (ctrl & 0x3F) as usize;
                assert!(count != 0);

                let color_index = input.read_le::<u8>().ok()? as usize;
                let color = palette[color_index % palette_count].as_ref();
                frame.extend(color.iter().cycle().take(count * 4));
            }
            (false, false) => {
                let count = (ctrl & 0x3F) as usize;
                assert!(count != 0);

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
