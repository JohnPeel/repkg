use std::fmt;

use binrw::binrw;

#[binrw]
pub struct Path {
    #[br(temp)]
    #[bw(calc = (path.len() + 1) as u16)]
    length: u16,
    #[br(count = length.max(1) - 1, try_map = String::from_utf8)]
    #[bw(map = |x: &String| x.as_bytes())]
    pub path: String,
    #[br(temp, assert(null_character == 0))]
    #[bw(calc = 0)]
    null_character: u8,
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.path, f)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.path, f)
    }
}
