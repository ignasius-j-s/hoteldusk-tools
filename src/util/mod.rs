use std::io::{Error, Read, Result, Seek, SeekFrom, Write};

mod color;
pub use color::Color;
mod io;
pub use io::{Endian, ReadExt, WriteExt};
mod lzss;
pub use lzss::decompress as lzss_decompress;

impl<T: Read> ReadExt for T {}
impl<T: Write> WriteExt for T {}

pub fn decompress<R: Read + Seek>(reader: &mut R) -> Result<Vec<u8>> {
    let magic = reader.read_bytes::<4>()?;
    let dst_size: u32 = reader.read_le()?;
    let src_size: u32 = reader.read_le()?;

    reader.seek(SeekFrom::Start(16))?;

    match &magic {
        [0x12, 0x3D, 0xDA, 1] => {
            let mut src_data = vec![0; src_size as usize];
            reader.read_exact(&mut src_data)?;
            let data = lzss_decompress(&src_data, dst_size as usize);
            Ok(data)
        }
        [0x12, 0x3D, 0xDA, 0] => {
            let mut data = vec![0; dst_size as usize];
            reader.read_exact(&mut data)?;
            Ok(data)
        }
        _ => Err(Error::other("uncompressed or unknown compression method")),
    }
}
