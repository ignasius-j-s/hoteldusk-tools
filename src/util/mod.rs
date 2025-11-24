use std::io::{Error, Read, Result};

mod color;
pub use color::Color;
mod io;
pub use io::{ReadEndian, ReadExt, WriteExt};
mod lzss;
pub use lzss::decompress as lzss_decompress;

pub fn decompress<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let magic = reader.read_bytes::<4>()?;

    match &magic {
        [0x12, 0x3D, 0xDA, flag @ 0..=1] => {
            let dst_size: u32 = reader.read_le()?;
            let src_size: u32 = reader.read_le()?;
            let _zero: u32 = reader.read_le()?;

            if *flag == 1 {
                let mut src_data = vec![0; src_size as usize];
                reader.read_exact(&mut src_data)?;
                let data = lzss_decompress(&src_data, dst_size as usize);
                Ok(data)
            } else {
                let mut data = vec![0; dst_size as usize];
                reader.read_exact(&mut data)?;
                Ok(data)
            }
        }
        [0x30, len1, len2, _] => {
            let output_len = u16::from_le_bytes([*len1, *len2]);
            let mut output = Vec::with_capacity(output_len as usize);

            while let Ok(ctrl) = reader.read_le::<u8>() {
                let flag = (ctrl & 0x80) != 0;
                let mut len = (ctrl & 0x7F) as usize;

                if flag {
                    len += 3;
                    let byte: u8 = reader.read_le()?;
                    output.extend(std::iter::repeat_n(byte, len));
                } else {
                    len += 1;
                    let current_len = output.len();
                    output.resize(current_len + len, 0);
                    reader.read_exact(&mut output[current_len..])?;
                }
            }

            Ok(output)
        }
        _ => Err(Error::other("uncompressed or unknown compression method")),
    }
}
