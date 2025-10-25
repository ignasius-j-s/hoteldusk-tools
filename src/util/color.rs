// rgba8888 color
pub struct Color([u8; 4]);

impl AsRef<[u8]> for Color {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Color {
    pub fn from_l8(l: u8) -> Color {
        Self([l, l, l, 0xFF])
    }

    pub fn from_bgr555(bytes: [u8; 2]) -> Self {
        let mut color = [0xFF; 4];
        let word = u16::from_le_bytes(bytes);

        color[0] = (word & 0x1F) as u8 * 8;
        color[1] = ((word >> 5) & 0x1F) as u8 * 8;
        color[2] = ((word >> 10) & 0x1F) as u8 * 8;

        color[0] += color[0] / 32;
        color[1] += color[1] / 32;
        color[2] += color[2] / 32;

        Self(color)
    }
}
