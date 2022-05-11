use std::fmt;

use binrw::{binread, until_eof};

use lpf::LuaPackFile;
use mpf::MeshPackFile;
use tpf::TexturePackFile;

pub use common::Path;
pub use lpf::{v0::Script as ScriptV0, v1::Script as ScriptV1, Global, Script};
pub use mpf::Mesh;
pub use tpf::{GameTexture, Palette, Texture, TextureFormat, TextureType};

#[binread]
#[br(little, magic = b"PPAK")]
pub struct Ppf {
    pub textures: TexturePackFile,
    pub meshes: MeshPackFile,
    pub scripts: LuaPackFile,
    #[br(parse_with = until_eof)]
    pub level: Vec<u8>,
}

impl fmt::Debug for Ppf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ppf")
            .field("textures", &self.textures)
            .field("meshes", &self.meshes)
            .field("scripts", &self.scripts)
            .field("level_size", &self.level.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn example() {
        let mut reader =
            BufReader::new(File::open("/home/john/psychonauts/workresource/pclevelpackfiles/wwma.ppf").unwrap());
        let _ppf = dbg!(Ppf::read(&mut reader).unwrap());
    }
}
