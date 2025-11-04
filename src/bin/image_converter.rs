use hoteldusk_tools::util::{Color, ReadExt, WriteExt, decompress};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Cursor, Read, Seek},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let files = std::env::args_os()
        .skip(1)
        .filter(|arg| std::fs::metadata(arg).is_ok_and(|md| md.is_file()))
        .collect::<Vec<_>>();

    if files.is_empty() {
        println!("Usage: image_converter file(s) ...");
        return Ok(());
    }

    for file in &files {
        let mut data = std::fs::read(file)?;
        let output = Path::new(file).with_extension("png");

        if let Ok(decompressed) = decompress(&mut data.as_slice()) {
            data = decompressed;
        }

        let mut reader = Cursor::new(&data);
        let header = reader.read_bytes::<16>()?;

        let (data, w, h) = if header == [0; 16] {
            let _w: u16 = reader.read_le()?;
            let _h: u16 = reader.read_le()?;
            let width: u16 = reader.read_le()?;
            let height: u16 = reader.read_le()?;
            let _flag: u16 = reader.read_le()?;
            let palette_count: u16 = reader.read_le()?;
            let palette_offset: u16 = reader.read_le()?;
            reader.seek_relative(2)?; // skip padding

            if reader.position() != palette_offset as u64 {
                continue;
            }

            let mut palette = Vec::with_capacity(palette_count as usize);
            for _ in 0..palette_count {
                let mut buf = [0; 2];
                reader.read_exact(&mut buf)?;
                let color = Color::from_rgb555(buf);
                palette.push(color);
            }

            let pixel_data_len = width as usize * height as usize;
            let mut pixel_data = Vec::with_capacity(pixel_data_len * size_of::<Color>());
            for _ in 0..pixel_data_len {
                let index: u8 = reader.read_le()?;
                pixel_data.write_bytes(palette[index as usize % palette_count as usize])?;
            }

            (pixel_data, width as u32, height as u32)
        } else {
            const TILE_W: usize = 8;
            const TILE_H: usize = 8;

            reader.seek_relative(-16)?;
            let _zero: u16 = reader.read_le()?;
            let palette_count: u16 = reader.read_le()?;
            let width: u16 = reader.read_le()?;
            let height: u16 = reader.read_le()?;
            let pixel_data_len: u32 = reader.read_le()?;
            let palette_len: u16 = reader.read_le()?;
            reader.seek_relative(2)?; // skip padding

            if palette_count * 2 != palette_len {
                continue;
            }

            if !width.is_multiple_of(TILE_W as u16) || !height.is_multiple_of(TILE_H as u16) {
                eprintln!("image dimension isnt divisible by tile dimension");
                continue;
            }

            let mut palette = Vec::with_capacity(palette_count as usize);
            let mut buf = [0; 2];
            for _ in 0..palette_count {
                reader.read_exact(&mut buf)?;
                let color = Color::from_rgb555(buf);
                palette.push(color);
            }

            let indexes = match palette_count {
                16 => {
                    let mut indexes = Vec::with_capacity(pixel_data_len as usize * 2);
                    for _ in 0..pixel_data_len {
                        let byte: u8 = reader.read_le()?;
                        indexes.push(byte & 0xF);
                        indexes.push(byte >> 4);
                    }
                    indexes
                }
                256 => {
                    let mut indexes = Vec::with_capacity(pixel_data_len as usize);
                    for _ in 0..pixel_data_len {
                        indexes.push(reader.read_le::<u8>()?);
                    }
                    indexes
                }
                other => {
                    eprintln!("unknown palette count format {other}");
                    continue;
                }
            };

            let tiles = indexes.chunks_exact(TILE_W * TILE_H);
            let tile_row_count = width as usize / TILE_W;

            let mut pixel_data = vec![0; width as usize * height as usize * size_of::<Color>()];
            for (i, tile) in tiles.enumerate() {
                let tile_x = (i % tile_row_count) * TILE_W;
                let tile_y = (i / tile_row_count) * TILE_H;

                for (j, index) in tile.iter().copied().enumerate() {
                    let x = (j % TILE_W) + tile_x;
                    let y = (j / TILE_W) + tile_y;
                    let pos = (y * width as usize + x) * 4;
                    pixel_data[pos..][..4].clone_from_slice(palette[index as usize].as_ref());
                }
            }

            (pixel_data, width as u32, height as u32)
        };

        let Ok(file) = File::create(output) else {
            continue;
        };
        let writer = BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let Ok(mut writer) = encoder.write_header() else {
            continue;
        };
        writer.write_image_data(&data).ok();
    }

    Ok(())
}
