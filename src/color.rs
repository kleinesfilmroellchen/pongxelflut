//! stolen from pixelpwnr

use random::Source;
use random::Value;

/// Color struct.
///
/// Represents a color with RGB values from 0 to 255.
#[derive(Copy, Clone)]
pub struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

impl Color {
    /// Constructor.
    ///
    /// The color channels must be between 0 and 255.
    pub fn from(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// Get a hexadecimal representation of the color,
    /// such as `FFFFFF` for white and `FF0000` for red.
    pub fn as_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }
}

impl Value for Color {
    fn read<S>(s: &mut S) -> Self
    where
        S: Source,
    {
        Self {
            r: s.read(),
            g: s.read(),
            b: s.read(),
            a: 0xff,
        }
    }
}
