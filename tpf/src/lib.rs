use std::{
    fmt,
    io::{Read, Seek, SeekFrom},
};

use binrw::{binrw, BinRead, BinReaderExt, BinResult, ReadOptions};

pub const DEFAULT_LANGUAGE: LanguageId = LanguageId::English;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u16)]
pub enum LanguageId {
    English = 0,
    French,
    German,
    Nonsense,
}

impl Default for LanguageId {
    fn default() -> Self {
        DEFAULT_LANGUAGE
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u32)]
pub enum TextureFormat {
    A8R8G8B8 = 0,
    R8G8B8,
    A4R4G4B4,
    A1R5G5B5,
    X1R5G5B5,
    R5G6B5,
    A8,
    L8,
    // FIXME: This name is weird?
    AL8,
    DXT1,
    DXT3,
    DXT5,
    V8U8,
    V16U16,
    PAL8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u32)]
pub enum TextureType {
    Bitmap = 0,
    Cubemap,
    VolumeMap,
    DepthBuffer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u32)]
pub enum PlayMode {
    Loop = 0,
    LoopOnce,
    LoopTail,
    Oscillate,
    OscillateOnce,
    OscillateOutOnce,
    OscillateBackOnce,
    Stop,
}

impl fmt::Display for PlayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub const DEFAULT_VERSION: Version = Version::V0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[binrw]
#[brw(repr = u16, magic = b"\xFD\xFD")]
pub enum Version {
    V0 = 0,
    V1,
}

impl Default for Version {
    fn default() -> Self {
        DEFAULT_VERSION
    }
}

#[derive(Debug)]
#[binrw]
pub struct AnimationInfo {
    #[br(assert(frame_count > 0, "Invalid frame count {}", frame_count))]
    pub frame_count: u32,
    #[br(assert(start_frame >= 0.0))]
    pub start_frame: f32,
    #[br(assert(loop_frame >= 0.0))]
    pub loop_frame: f32,
    pub start_time: f32,
    #[br(assert(frame_rate >= 0.0))]
    pub frame_rate: f32,
    pub play_mode: PlayMode,
    #[br(map = |x: u8| x != 0)]
    #[bw(map = |x: &bool| *x as u8)]
    #[brw(pad_after = 3)]
    pub playing: bool,
}

#[binrw]
pub struct Palette {
    #[br(temp)]
    #[bw(calc = data.is_some() as u16)]
    has_data: u16,

    #[br(if(has_data != 0))]
    pub data: Option<[u32; 0x100]>,
}

impl fmt::Debug for Palette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Palette").finish()
    }
}

#[binrw]
pub struct Texture {
    #[brw(pad_before = 4)]
    pub format: TextureFormat,
    pub type_: TextureType,
    pub flags: u32,
    #[br(try_map = |x: u32| x.try_into())]
    #[bw(map = |x: &usize| *x as u32)]
    pub width: usize,
    #[br(try_map = |x: u32| x.try_into())]
    #[bw(map = |x: &usize| *x as u32)]
    pub height: usize,
    #[brw(pad_after = 16)]
    #[br(try_map = |x: u32| x.try_into().map(|mipmaps| calculate_mipmaps(mipmaps, width, height)))]
    #[bw(map = |x: &usize| *x as u32)]
    pub mipmaps: usize,
    #[br(if(format == TextureFormat::PAL8))]
    pub palette: Option<Palette>,
    #[br(count = calculate_texture_size(format, type_, width, height, mipmaps))]
    pub data: Vec<u8>,
}

impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Texture")
            .field("format", &self.format)
            .field("type", &self.type_)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("mipmaps", &self.mipmaps)
            .field("palette", &self.palette)
            .field("size", &self.data.len())
            .finish()
    }
}

pub mod v0 {
    use std::fmt;

    use common::Path;

    use super::*;

    #[binrw]
    pub struct GameTexture {
        pub element_id: u32,
        #[br(assert(texture_handle > 0))]
        pub texture_handle: u32,
        pub palette_handle: u32,
        pub path_pointer: u32,
        pub animation_info_pointer: u32,
        pub density: f32,
        pub visual_importance: u32,
        pub memory_importance: u32,
        pub unknown0: u32,
        pub flags: u32,
        #[br(if(path_pointer != 0))]
        pub path: Option<Path>,
        #[br(if(animation_info_pointer != 0))]
        pub animation_info: Option<AnimationInfo>,
        #[br(count = animation_info.as_ref().map(|x| x.frame_count).unwrap_or(1))]
        pub textures: Vec<Texture>,
    }

    impl fmt::Debug for GameTexture {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("GameTexture")
                .field("path", &self.path)
                .field("animation_info", &self.animation_info)
                .field("textures", &self.textures)
                .finish()
        }
    }
}

pub mod v1 {
    use super::*;

    #[derive(Debug)]
    #[binrw]
    pub struct GameTexture {
        #[br(try_map = |x: u32| x.try_into())]
        #[bw(map = |x: &usize| *x as u32)]
        pub size: usize,
        pub game_texture: v0::GameTexture,
    }
}

#[derive(Debug)]
#[binrw]
pub enum GameTexture {
    V0(v0::GameTexture),
    #[brw(magic = b" XT1")]
    V1(v1::GameTexture),
}

#[derive(Debug)]
#[binrw]
pub struct Language {
    pub id: LanguageId,
    #[br(try_map = |x: u32| x.try_into())]
    #[bw(map = |x: &usize| *x as u32)]
    pub size: usize,
    #[br(try_map = |x: u16| x.try_into())]
    #[bw(map = |x: &usize| *x as u16)]
    pub count: usize,
    #[br(count = count)]
    pub game_textures: Vec<GameTexture>,
}

#[binrw]
#[brw(little)]
pub struct TexturePackFile {
    #[br(try)]
    pub version: Option<Version>,
    #[br(parse_with = languages_parser)]
    pub languages: Vec<Language>,
    #[br(temp)]
    #[bw(calc = game_textures.len() as u16)]
    count: u16,
    #[br(count = count)]
    pub game_textures: Vec<GameTexture>,
}

impl fmt::Debug for TexturePackFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tpf")
            .field("version", &self.version)
            .field("languages", &self.languages)
            .field("game_textures", &self.game_textures)
            .finish()
    }
}

impl TextureFormat {
    pub fn compressed(&self) -> bool {
        matches!(*self, TextureFormat::DXT1 | TextureFormat::DXT3 | TextureFormat::DXT5)
    }

    pub fn block_size(&self) -> usize {
        match *self {
            TextureFormat::DXT1 => 8,
            TextureFormat::DXT3 | TextureFormat::DXT5 => 16,
            _ => unimplemented!(),
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match *self {
            TextureFormat::A8R8G8B8 | TextureFormat::V16U16 => 4,
            TextureFormat::R8G8B8 => 3,
            TextureFormat::A4R4G4B4
            | TextureFormat::A1R5G5B5
            | TextureFormat::X1R5G5B5
            | TextureFormat::R5G6B5
            | TextureFormat::V8U8 => 2,
            TextureFormat::L8 | TextureFormat::A8 | TextureFormat::AL8 | TextureFormat::PAL8 => 1,
            _ => unimplemented!(),
        }
    }
}

fn calculate_mipmaps(mipmaps: usize, width: usize, height: usize) -> usize {
    let mut mipmaps = mipmaps;
    if mipmaps == 0 {
        let (mut width, mut height) = (width, height);
        while width > 0 && height > 0 {
            width >>= 1;
            height >>= 1;
            mipmaps += 1;
        }
    }
    mipmaps
}

fn calculate_texture_size(
    format: TextureFormat,
    type_: TextureType,
    width: usize,
    height: usize,
    mipmap_levels: usize,
) -> usize {
    match type_ {
        TextureType::Bitmap => {
            let mut size = 0;
            let mut width = width;
            let mut height = height;
            let compressed = format.compressed();

            for _ in 0..mipmap_levels {
                let mipmap_size = if compressed {
                    ((width + 3) >> 2).max(1) * ((height + 3) >> 2).max(1) * format.block_size()
                } else {
                    width * height * format.bytes_per_pixel()
                };

                width >>= 1;
                height >>= 1;
                size += mipmap_size;
            }
            size
        }
        TextureType::Cubemap => 6 * calculate_texture_size(format, TextureType::Bitmap, width, height, mipmap_levels),
        TextureType::DepthBuffer => unimplemented!(),
        TextureType::VolumeMap => unimplemented!(),
    }
}

fn languages_parser<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, _: ()) -> BinResult<Vec<Language>> {
    let mut languages = Vec::with_capacity(LanguageId::Nonsense as usize);
    let mut magic: u16 = reader.read_be()?;
    while magic == 0xFFFF {
        languages.push(Language::read_options(reader, ro, ())?);
        magic = reader.read_be()?;
    }
    reader.seek(SeekFrom::Current(-2))?;
    Ok(languages)
}
