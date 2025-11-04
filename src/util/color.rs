// rgba8888 color
#[derive(Clone, Copy)]
pub struct Color([u8; 4]);

impl Color {
    pub fn from_rgb555(bytes: [u8; 2]) -> Self {
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

    pub fn r(&self) -> u8 {
        self.0[0]
    }

    pub fn g(&self) -> u8 {
        self.0[1]
    }

    pub fn b(&self) -> u8 {
        self.0[2]
    }

    pub fn a(&self) -> u8 {
        self.0[3]
    }

    pub fn multiply(&self, other: Color) -> Color {
        const MAX: u16 = 0xFF;

        Color([
            (u16::from(self.r()) * u16::from(other.r()) / MAX) as u8,
            (u16::from(self.g()) * u16::from(other.g()) / MAX) as u8,
            (u16::from(self.b()) * u16::from(other.b()) / MAX) as u8,
            (u16::from(self.a()) * u16::from(other.a()) / MAX) as u8,
        ])
    }
}

impl From<[u8; 4]> for Color {
    fn from(value: [u8; 4]) -> Self {
        Self(value)
    }
}

impl AsRef<[u8]> for Color {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
