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

pub trait Size {
    fn size(&self) -> usize;
}

impl Size for u32 {
    fn size(&self) -> usize {
        4
    }
}

impl<T: Size> Size for Option<T> {
    fn size(&self) -> usize {
        self.as_ref().map(|x| x.size()).unwrap_or(0)
    }
}

impl<const N: usize> Size for [u32; N] {
    fn size(&self) -> usize {
        4 * N
    }
}

impl<T: Size> Size for Vec<T> {
    fn size(&self) -> usize {
        self.iter().map(|x| x.size()).sum()
    }
}

impl Size for Path {
    fn size(&self) -> usize {
        3 + self.path.len()
    }
}
