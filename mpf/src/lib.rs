use std::fmt;

use binrw::binrw;

use common::Path;

#[binrw]
pub struct Mesh {
    #[brw(pad_after = 2)]
    pub path: Path,
    #[br(temp)]
    #[bw(calc = data.len() as u32)]
    size: u32,
    #[br(count = size)]
    // TODO: These are just blobs for now. The format needs reversing.
    pub data: Vec<u8>,
}

impl fmt::Debug for Mesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mesh")
            .field("path", &self.path)
            .field("size", &self.data.len())
            .finish()
    }
}

#[binrw]
#[brw(little, magic = b"MPAK")]
pub struct MeshPackFile {
    #[br(temp)]
    #[bw(calc = meshes.len() as u16)]
    count: u16,
    #[br(count = count)]
    pub meshes: Vec<Mesh>,
}

impl fmt::Debug for MeshPackFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Vec::fmt(&self.meshes, f)
    }
}
