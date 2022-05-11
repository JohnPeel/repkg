use std::fmt;

use binrw::binrw;
use common::Path;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u16, magic = b"\xFC\xFC")]
pub enum Version {
    V0 = 0,
    V1,
}

impl Default for Version {
    fn default() -> Self {
        Self::V0
    }
}

#[binrw]
pub struct Global {
    pub path: Path,
    #[br(temp)]
    #[bw(calc = data.len() as u32)]
    size: u32,
    #[br(count = size)]
    pub data: Vec<u8>,
}

impl fmt::Debug for Global {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Global")
            .field("path", &self.path)
            .field("size", &self.data.len())
            .finish()
    }
}

pub mod v0 {
    use super::*;

    #[binrw]
    pub struct Script {
        #[br(temp)]
        #[bw(calc = data.len() as u32)]
        size: u32,

        #[br(count = size)]
        pub data: Vec<u8>,
    }

    impl fmt::Debug for Script {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Script").field("size", &self.data.len()).finish()
        }
    }
}

pub mod v1 {
    use super::*;

    #[binrw]
    pub struct Script {
        pub path: Path,
        pub script: v0::Script,
    }

    impl fmt::Debug for Script {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Script")
                .field("path", &self.path)
                .field("size", &self.script.data.len())
                .finish()
        }
    }
}

#[derive(Debug)]
#[binrw]
#[br(import(version: Version))]
pub enum Script {
    #[br(assert(version == Version::V0))]
    V0(v0::Script),
    #[br(assert(version == Version::V1))]
    V1(v1::Script),
}

#[binrw]
#[brw(little)]
pub struct LuaPackFile {
    #[br(try)]
    pub version: Option<Version>,

    #[br(temp)]
    #[bw(calc = globals.len() as u16)]
    global_count: u16,
    #[br(count = global_count)]
    pub globals: Vec<Global>,

    #[br(temp)]
    #[bw(calc = scripts.len() as u16)]
    script_count: u16,

    #[br(args { count: script_count.into(), inner: (version.unwrap_or_default(),) })]
    pub scripts: Vec<Script>,
}

impl fmt::Debug for LuaPackFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LuaPackFile")
            .field("version", &self.version)
            .field("globals", &self.globals)
            .field("scripts", &self.scripts)
            .finish()
    }
}
