use std::io::{Read, Result, Write};

pub trait Endian: Sized {
    fn read_le<R: Read>(reader: &mut R) -> Result<Self>;
    fn read_be<R: Read>(reader: &mut R) -> Result<Self>;
    fn write_le<W: Write>(self, writer: &mut W) -> Result<()>;
    fn write_be<W: Write>(self, writer: &mut W) -> Result<()>;
}

macro_rules! impl_endian {
    ($($type:ty),*) => ($(
        impl Endian for $type {
            #[inline]
            fn read_le<R: Read>(reader: &mut R) -> Result<Self> {
                let mut buf = [0; size_of::<$type>()];
                reader.read_exact(&mut buf)?;
                Ok(<$type>::from_le_bytes(buf))
            }

            #[inline]
            fn read_be<R: Read>(reader: &mut R) -> Result<Self> {
                let mut buf = [0; size_of::<$type>()];
                reader.read_exact(&mut buf)?;
                Ok(<$type>::from_be_bytes(buf))
            }

            #[inline]
            fn write_le<W: Write>(self, writer: &mut W) -> Result<()> {
                let buf = <$type>::to_le_bytes(self);
                writer.write_all(&buf)?;
                Ok(())
            }

            #[inline]
            fn write_be<W: Write>(self, writer: &mut W) -> Result<()> {
                let buf = <$type>::to_be_bytes(self);
                writer.write_all(&buf)?;
                Ok(())
            }
        }
    )*)
}

impl_endian!(i8, i16, i32, i64);
impl_endian!(u8, u16, u32, u64);

pub trait ReadExt: Read + Sized {
    #[inline]
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0_u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    #[inline]
    fn read_le<T: Endian>(&mut self) -> Result<T> {
        T::read_le(self)
    }

    #[inline]
    fn read_be<T: Endian>(&mut self) -> Result<T> {
        T::read_be(self)
    }
}

pub trait WriteExt: Write + Sized {
    #[inline]
    fn write_bytes<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<()> {
        self.write_all(bytes.as_ref())
    }

    #[inline]
    fn write_le<T: Endian>(&mut self, value: T) -> Result<()> {
        T::write_le(value, self)
    }

    #[inline]
    fn write_be<T: Endian>(&mut self, value: T) -> Result<()> {
        T::write_be(value, self)
    }
}
