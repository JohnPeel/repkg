use std::mem::size_of;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

pub const MAGIC: u32 = 0x20534444;

bitflags! {
    #[derive(Default, Serialize, Deserialize)]
    pub struct PixelFormatFlags: u32 {
        const ALPHAPIXELS = 0x00000001;
        const ALPHA = 0x00000002;
        const FOURCC = 0x00000004;
        const PAL8 = 0x00000020;
        const PAL8A = PixelFormatFlags::PAL8.bits | PixelFormatFlags::ALPHAPIXELS.bits;
        const RGB = 0x00000040;
        const RGBA = PixelFormatFlags::RGB.bits | PixelFormatFlags::ALPHAPIXELS.bits;
        const YUV = 0x00000200;
        const LUMINANCE = 0x00020000;
        const LUMINANCEA = PixelFormatFlags::LUMINANCE.bits | PixelFormatFlags::ALPHAPIXELS.bits;
        const BUMPLUMINANCE = 0x00040000;
        const BUMPDUDV = 0x00080000;
    }
}

pub const FOURCC: PixelFormatFlags = PixelFormatFlags::FOURCC;
pub const RGB: PixelFormatFlags = PixelFormatFlags::RGB;
pub const RGBA: PixelFormatFlags = PixelFormatFlags::RGBA;
pub const LUMINANCE: PixelFormatFlags = PixelFormatFlags::LUMINANCE;
pub const LUMINANCEA: PixelFormatFlags = PixelFormatFlags::LUMINANCEA;
pub const ALPHA: PixelFormatFlags = PixelFormatFlags::ALPHA;
pub const PAL8: PixelFormatFlags = PixelFormatFlags::PAL8;
pub const PAL8A: PixelFormatFlags = PixelFormatFlags::PAL8A;
pub const BUMPDUDV: PixelFormatFlags = PixelFormatFlags::BUMPDUDV;
pub const BUMPLUMINANCE: PixelFormatFlags = PixelFormatFlags::BUMPLUMINANCE;

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PixelFormat {
    pub _size: u32,
    pub flags: PixelFormatFlags,
    pub four_cc: [u8; 4],
    pub rgb_bit_count: u32,
    pub r_bit_mask: u32,
    pub g_bit_mask: u32,
    pub b_bit_mask: u32,
    pub a_bit_mask: u32,
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self {
            _size: size_of::<Self>() as u32,
            flags: Default::default(),
            four_cc: Default::default(),
            rgb_bit_count: Default::default(),
            r_bit_mask: Default::default(),
            g_bit_mask: Default::default(),
            b_bit_mask: Default::default(),
            a_bit_mask: Default::default(),
        }
    }
}

impl PixelFormat {
    pub const DXT1: PixelFormat = PixelFormat::from_four_cc(*b"DXT1");
    pub const DXT2: PixelFormat = PixelFormat::from_four_cc(*b"DXT2");
    pub const DXT3: PixelFormat = PixelFormat::from_four_cc(*b"DXT3");
    pub const DXT4: PixelFormat = PixelFormat::from_four_cc(*b"DXT4");
    pub const DXT5: PixelFormat = PixelFormat::from_four_cc(*b"DXT5");
    pub const BC4_UNORM: PixelFormat = PixelFormat::from_four_cc(*b"BC4U");
    pub const BC4_SNORM: PixelFormat = PixelFormat::from_four_cc(*b"BC4S");
    pub const BC5_UNORM: PixelFormat = PixelFormat::from_four_cc(*b"BC5U");
    pub const BC5_SNORM: PixelFormat = PixelFormat::from_four_cc(*b"BC5S");
    pub const R8G8_B8G8: PixelFormat = PixelFormat::from_four_cc(*b"RGBG");
    pub const G8R8_G8B8: PixelFormat = PixelFormat::from_four_cc(*b"GRGB");
    pub const YUY2: PixelFormat = PixelFormat::from_four_cc(*b"YUY2");
    pub const UYVY: PixelFormat = PixelFormat::from_four_cc(*b"UYVY");

    pub const A8R8G8B8: PixelFormat =
        PixelFormat::from_tuple((RGBA, 32, 0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000));
    pub const X8R8G8B8: PixelFormat = PixelFormat::from_tuple((RGB, 32, 0x00ff0000, 0x0000ff00, 0x000000ff, 0));
    pub const A8B8G8R8: PixelFormat =
        PixelFormat::from_tuple((RGBA, 32, 0x000000ff, 0x0000ff00, 0x00ff0000, 0xff000000));
    pub const X8B8G8R8: PixelFormat = PixelFormat::from_tuple((RGB, 32, 0x000000ff, 0x0000ff00, 0x00ff0000, 0));
    pub const G16R16: PixelFormat = PixelFormat::from_tuple((RGB, 32, 0x0000ffff, 0xffff0000, 0, 0));
    pub const R5G6B5: PixelFormat = PixelFormat::from_tuple((RGB, 16, 0xf800, 0x07e0, 0x001f, 0));
    pub const A1R5G5B5: PixelFormat = PixelFormat::from_tuple((RGBA, 16, 0x7c00, 0x03e0, 0x001f, 0x8000));
    pub const X1R5G5B5: PixelFormat = PixelFormat::from_tuple((RGB, 16, 0x7c00, 0x03e0, 0x001f, 0));
    pub const A4R4G4B4: PixelFormat = PixelFormat::from_tuple((RGBA, 16, 0x0f00, 0x00f0, 0x000f, 0xf000));
    pub const X4R4G4B4: PixelFormat = PixelFormat::from_tuple((RGB, 16, 0x0f00, 0x00f0, 0x000f, 0));
    pub const R8G8B8: PixelFormat = PixelFormat::from_tuple((RGB, 24, 0xff0000, 0x00ff00, 0x0000ff, 0));
    pub const A8R3G3B2: PixelFormat = PixelFormat::from_tuple((RGBA, 16, 0x00e0, 0x001c, 0x0003, 0xff00));
    pub const R3G3B2: PixelFormat = PixelFormat::from_tuple((RGB, 8, 0xe0, 0x1c, 0x03, 0));
    pub const A4L4: PixelFormat = PixelFormat::from_tuple((LUMINANCEA, 8, 0x0f, 0, 0, 0xf0));
    pub const L8: PixelFormat = PixelFormat::from_tuple((LUMINANCE, 8, 0xff, 0, 0, 0));
    pub const L16: PixelFormat = PixelFormat::from_tuple((LUMINANCE, 16, 0xffff, 0, 0, 0));
    pub const A8L8: PixelFormat = PixelFormat::from_tuple((LUMINANCEA, 16, 0x00ff, 0, 0, 0xff00));
    pub const A8L8_ALT: PixelFormat = PixelFormat::from_tuple((LUMINANCEA, 8, 0x00ff, 0, 0, 0xff00));
    pub const L8_NVTT1: PixelFormat = PixelFormat::from_tuple((RGB, 8, 0xff, 0, 0, 0));
    pub const L16_NVTT1: PixelFormat = PixelFormat::from_tuple((RGB, 16, 0xffff, 0, 0, 0));
    pub const A8L8_NVTT1: PixelFormat = PixelFormat::from_tuple((RGBA, 16, 0x00ff, 0, 0, 0xff00));
    pub const A8: PixelFormat = PixelFormat::from_tuple((ALPHA, 8, 0, 0, 0, 0xff));
    pub const V8U8: PixelFormat = PixelFormat::from_tuple((BUMPDUDV, 16, 0x00ff, 0xff00, 0, 0));
    pub const Q8W8V8U8: PixelFormat =
        PixelFormat::from_tuple((BUMPDUDV, 32, 0x000000ff, 0x0000ff00, 0x00ff0000, 0xff000000));
    pub const V16U16: PixelFormat = PixelFormat::from_tuple((BUMPDUDV, 32, 0x0000ffff, 0xffff0000, 0, 0));

    #[deprecated = "Use DX10 extension to avoid reversal issue."]
    pub const A2R10G10B10: PixelFormat =
        PixelFormat::from_tuple((RGBA, 32, 0x000003ff, 0x000ffc00, 0x3ff00000, 0xc0000000));
    #[deprecated = "Use DX10 extension to avoid reversal issue."]
    pub const A2B10G10R10: PixelFormat =
        PixelFormat::from_tuple((RGBA, 32, 0x3ff00000, 0x000ffc00, 0x000003ff, 0xc0000000));

    pub const A2W10V10U10: PixelFormat =
        PixelFormat::from_tuple((BUMPDUDV, 32, 0x3ff00000, 0x000ffc00, 0x000003ff, 0xc0000000));
    pub const L6V5U5: PixelFormat = PixelFormat::from_tuple((BUMPLUMINANCE, 16, 0x001f, 0x03e0, 0xfc00, 0));
    pub const X8L8V8U8: PixelFormat =
        PixelFormat::from_tuple((BUMPLUMINANCE, 32, 0x000000ff, 0x0000ff00, 0x00ff0000, 0));

    #[inline]
    pub const fn from_tuple(
        (flags, rgb_bit_count, r_bit_mask, g_bit_mask, b_bit_mask, a_bit_mask): (
            PixelFormatFlags,
            u32,
            u32,
            u32,
            u32,
            u32,
        ),
    ) -> Self {
        Self {
            _size: size_of::<Self>() as u32,
            flags,
            four_cc: [0; 4],
            rgb_bit_count,
            r_bit_mask,
            g_bit_mask,
            b_bit_mask,
            a_bit_mask,
        }
    }

    #[inline]
    pub const fn from_four_cc(four_cc: [u8; 4]) -> Self {
        Self {
            _size: size_of::<Self>() as u32,
            flags: FOURCC,
            four_cc,
            rgb_bit_count: 0,
            r_bit_mask: 0,
            g_bit_mask: 0,
            b_bit_mask: 0,
            a_bit_mask: 0,
        }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct HeaderFlags: u32 {
        const CAPS        = 0x00000001;
        const HEIGHT      = 0x00000002;
        const WIDTH       = 0x00000004;
        const PITCH       = 0x00000008;
        const PIXELFORMAT = 0x00001000;
        const MIPMAPCOUNT = 0x00020000;
        const LINEARSIZE  = 0x00080000;
        const DEPTH       = 0x00800000;
    }

    #[derive(Serialize, Deserialize)]
    pub struct SurfaceFlags: u32 {
        const COMPLEX = 0x00000008;
        const TEXTURE = 0x00001000;
        const MIPMAP  = 0x00400000;
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Caps2: u32 {
        const CUBEMAP   = 0x00000200;
        const POSITIVEX = 0x00000400;
        const NEGATIVEX = 0x00000800;
        const POSITIVEY = 0x00001000;
        const NEGATIVEY = 0x00002000;
        const POSITIVEZ = 0x00004000;
        const NEGATIVEZ = 0x00008000;
        const VOLUME    = 0x00200000;
    }
}

pub const HEADER_FLAGS_TEXTURE: HeaderFlags = HeaderFlags {
    bits: HeaderFlags::CAPS.bits | HeaderFlags::HEIGHT.bits | HeaderFlags::WIDTH.bits | HeaderFlags::PIXELFORMAT.bits,
};
pub const HEADER_FLAGS_MIPMAP: HeaderFlags = HeaderFlags::MIPMAPCOUNT;
pub const HEADER_FLAGS_VOLUME: HeaderFlags = HeaderFlags::DEPTH;
pub const HEADER_FLAGS_PITCH: HeaderFlags = HeaderFlags::PITCH;
pub const HEADER_FLAGS_LINEARSIZE: HeaderFlags = HeaderFlags::LINEARSIZE;

impl Default for HeaderFlags {
    fn default() -> Self {
        HEADER_FLAGS_TEXTURE
    }
}

pub const SURFACE_FLAGS_MIPMAP: SurfaceFlags = SurfaceFlags {
    bits: SurfaceFlags::COMPLEX.bits | SurfaceFlags::MIPMAP.bits,
};
pub const SURFACE_FLAGS_TEXTURE: SurfaceFlags = SurfaceFlags::TEXTURE;
pub const SURFACE_FLAGS_CUBEMAP: SurfaceFlags = SurfaceFlags::COMPLEX;

impl Default for SurfaceFlags {
    fn default() -> Self {
        SURFACE_FLAGS_TEXTURE
    }
}

pub const CUBEMAP_POSITIVEX: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::POSITIVEX.bits,
};
pub const CUBEMAP_NEGATIVEX: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::NEGATIVEX.bits,
};
pub const CUBEMAP_POSITIVEY: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::POSITIVEY.bits,
};
pub const CUBEMAP_NEGATIVEY: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::NEGATIVEY.bits,
};
pub const CUBEMAP_POSITIVEZ: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::POSITIVEZ.bits,
};
pub const CUBEMAP_NEGATIVEZ: Caps2 = Caps2 {
    bits: Caps2::CUBEMAP.bits | Caps2::NEGATIVEZ.bits,
};
pub const CUBEMAP_ALLFACES: Caps2 = Caps2 {
    bits: CUBEMAP_POSITIVEX.bits
        | CUBEMAP_NEGATIVEX.bits
        | CUBEMAP_POSITIVEY.bits
        | CUBEMAP_NEGATIVEY.bits
        | CUBEMAP_POSITIVEZ.bits
        | CUBEMAP_NEGATIVEZ.bits,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Header {
    pub _size: u32,
    pub header_flags: HeaderFlags,
    pub height: u32,
    pub width: u32,
    pub pitch_or_linear_size: u32,
    pub depth: u32,
    pub mip_map_count: u32,
    pub _reserved1: [u32; 11],
    pub pixel_format: PixelFormat,
    pub surface_flags: SurfaceFlags,
    pub caps2: Caps2,
    pub _caps3: u32,
    pub _caps4: u32,
    pub _reserved2: u32,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            _size: size_of::<Self>() as u32,
            header_flags: Default::default(),
            height: Default::default(),
            width: Default::default(),
            pitch_or_linear_size: Default::default(),
            depth: Default::default(),
            mip_map_count: Default::default(),
            _reserved1: Default::default(),
            pixel_format: Default::default(),
            surface_flags: Default::default(),
            caps2: Default::default(),
            _caps3: Default::default(),
            _caps4: Default::default(),
            _reserved2: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn proper_size() {
        assert_eq!(32, size_of::<PixelFormat>(), "PixelFormat size mismatch.");
        assert_eq!(124, size_of::<Header>(), "Header size mismatch.");
    }
}
