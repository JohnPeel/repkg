use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    mem::size_of,
    path::{Path, PathBuf},
};

use clap::{AppSettings, Clap};

use dds::PixelFormat;
use pkg::{Zpkg, ZpkgFile};
use ppf::{Ppf, Script, Texture, TextureFormat, TextureType, DEFAULT_LANGUAGE};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clap)]
#[clap(version = "0.0.1", author = "John Peel <john@dgby.org>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short = 'v', long)]
    verbose: bool,
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Decompress {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    ExtractPkg {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    CreatePpf {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    ExtractPpf {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    MeshInfo {
        #[clap(parse(from_os_str))]
        input: PathBuf,
    },
}

#[allow(unused)]
fn decompress(input: &[u8]) -> nom::IResult<&[u8], Vec<u8>> {
    use nom::bytes::complete::*;
    use nom::number::complete::*;

    let (input, _) = tag("ZLIB")(input)?;
    let (input, version) = le_u32(input)?;
    let (input, decompressed_size) = le_u32(input)?;
    let (input, compressed_size) = le_u32(input)?;
    let (input, compressed_data) = take(compressed_size)(input)?;
    assert_eq!(0, input.len());

    use flate2::read::ZlibDecoder;

    let mut buffer = Vec::with_capacity(decompressed_size as usize);
    let mut decoder = ZlibDecoder::new(compressed_data);
    decoder.read_to_end(&mut buffer).expect("Unable to decode zlib.");
    assert_eq!(decompressed_size as usize, buffer.len());

    Ok((input, buffer))
}

trait DdsHeader {
    fn dds_header(&self) -> Result<Vec<u8>, BoxError>;
}

impl<'a> DdsHeader for Texture<'a> {
    fn dds_header(&self) -> Result<Vec<u8>, BoxError> {
        let mut header: dds::Header = dds::Header {
            height: self.height as u32,
            width: self.width as u32,
            depth: 1,
            mip_map_count: self.mipmap_levels as u32,
            pixel_format: match self.format {
                TextureFormat::A8R8G8B8 => PixelFormat::A8R8G8B8,
                TextureFormat::R8G8B8 => PixelFormat::R8G8B8, // FIXME: OpenGL types point to X8R8G8B8, but LoadTextureFromDDSStream points to R8G8B8
                TextureFormat::A4R4G4B4 => PixelFormat::A4R4G4B4,
                TextureFormat::A1R5G5B5 => PixelFormat::A1R5G5B5,
                TextureFormat::X1R5G5B5 => PixelFormat::X1R5G5B5,
                TextureFormat::R5G6B5 => PixelFormat::R5G6B5,
                TextureFormat::A8 => PixelFormat::A8,
                TextureFormat::L8 => PixelFormat::L8, // FIXME: LoadTextureFromDDSStream loads this from A8 dds header.
                TextureFormat::AL8 => unimplemented!(), // FIXME: Possibly A8L8_ALT or A4L4.
                TextureFormat::DXT1 => PixelFormat::DXT1,
                TextureFormat::DXT3 => PixelFormat::DXT3,
                TextureFormat::DXT5 => PixelFormat::DXT5,
                TextureFormat::V8U8 => PixelFormat::V8U8,
                TextureFormat::V16U16 => PixelFormat::V16U16,
                TextureFormat::PAL8 => PixelFormat::from_tuple((dds::PAL8, 0, 0, 0, 0, 0)), // FIXME: This is wrong, according to the game generated PAL8 textures.
            },
            ..Default::default()
        };

        if self.mipmap_levels > 1 {
            header.header_flags.insert(dds::HEADER_FLAGS_MIPMAP);
            header.surface_flags.insert(dds::SURFACE_FLAGS_MIPMAP);
        }

        if self.type_ == TextureType::Cubemap {
            header.surface_flags.insert(dds::SURFACE_FLAGS_CUBEMAP);
            header.caps2 = dds::CUBEMAP_ALLFACES;
        }

        header.pitch_or_linear_size = if self.format.compressed() {
            header.header_flags.insert(dds::HEADER_FLAGS_LINEARSIZE);
            let (width, height) = (self.width as u32, self.height as u32);
            ((width + 3) >> 2).max(1) * ((height + 3) >> 2).max(1) * self.format.block_size() as u32
        } else {
            header.header_flags.insert(dds::HEADER_FLAGS_PITCH);
            (self.width as u32 * header.pixel_format.rgb_bit_count + 7) / 8
        };

        Ok(bincode::serialize(&header)?)
    }
}

fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, BoxError> {
    let file = File::open(path.as_ref())?;
    let metadata = file.metadata()?;
    let mut reader = BufReader::new(file);

    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn write_file<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), BoxError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(data)?;
    Ok(())
}

mod plb {
    use std::convert::Infallible;

    use nom::{bytes::complete::*, combinator::*, multi::*, number::complete::*, sequence::*, IResult};

    pub fn le_u32_as_usize(input: &[u8]) -> IResult<&[u8], usize> {
        map_res(le_u32, |x| Result::<_, Infallible>::Ok(x as usize))(input)
    }

    pub fn le_u32_length_c_string(input: &[u8]) -> IResult<&[u8], &str> {
        let (input, length) = le_u32(input)?;
        assert!(length > 0);
        map_res(terminated(take(length - 1), tag("\0")), std::str::from_utf8)(input)
    }

    pub fn name(input: &[u8]) -> IResult<&[u8], (&str, u8, u8)> {
        tuple((le_u32_length_c_string, le_u8, le_u8))(input)
    }

    pub fn joint(input: &[u8], version: u32, mut some_flag: u8) -> IResult<&[u8], ()> {
        let (input, name) = le_u32_length_c_string(input)?;
        log::info!("joint_name = {:?}", name);
        let (input, _some_vec0) = many_m_n(3, 3, le_f32)(input)?;
        let (input, some_vec1) = many_m_n(3, 3, le_f32)(input)?;
        some_flag &= 0xfe;

        if some_vec1 == vec![-1.0, -1.0, -1.0] {
            some_flag |= 8;
        }

        let (input, _) = if version > 0x126 {
            let (input, unknown0) = le_u16(input)?;
            log::info!("unknown0 = {}", unknown0);
            (input, Some(unknown0))
        } else {
            (input, None)
        };

        let (input, unknown1) = le_u32(input)?;
        log::info!("unknown1 = {}", unknown1);

        some_flag = some_flag & 0xfb | ((unknown1 != 0) as u8 * 4);

        let (input, ()) = if some_flag >> 2 & 1 != 0 {
            let (input, index) = le_u16(input)?;
            log::info!("joint_index = {}", index);
            let (input, _joint) = joint(input, version, some_flag)?;
            (input, ())
        } else {
            (input, ())
        };

        Ok((input, ()))
    }

    pub fn skeleton(input: &[u8], version: u32) -> IResult<&[u8], ()> {
        let (input, skeleton_name) = le_u32_length_c_string(input)?;
        log::info!("skeleton_name = {:?}", skeleton_name);
        let (input, joint_count) = le_u32(input)?;
        log::info!("joint_count = {}", joint_count);
        let (input, _unknown1) = le_u16(input)?;
        let (input, _joint) = joint(input, version, 0)?;
        Ok((input, ()))
    }

    type Mesh<'a> = (&'a str, Vec<Vec<f32>>, usize);

    pub fn mesh(input: &[u8], version: u32) -> IResult<&[u8], Mesh<'_>> {
        let (input, name) = le_u32_length_c_string(input)?;
        log::info!("mesh_name = {}", name);
        let (input, a_lot_of_vecs) = many_m_n(5, 5, many_m_n(3, 3, le_f32))(input)?;
        let (input, some_count) = le_u32_as_usize(input)?;
        log::info!("some_count = {}", some_count);
        let (input, maybe_flags) = le_u32(input)?;
        log::info!("maybe_flags = 0x{:x}", maybe_flags);

        let (input, _some_strings) = if maybe_flags & 1 == 1 {
            let (input, some_strings) = many_m_n(2, 2, le_u32_length_c_string)(input)?;
            log::info!("some_strings = {:?}", some_strings);
            (input, Some(some_strings))
        } else {
            (input, None)
        };

        let (input, _something) = if maybe_flags & 2 == 2 {
            let (input, lod_count) = le_u8(input)?;
            let (input, something) = take((lod_count - 1) as usize * 4)(input)?;
            log::info!("something = {:?}", &something[..10]);
            (input, Some(something))
        } else {
            (input, None)
        };

        let (input, some_count2) = le_u32_as_usize(input)?;
        log::info!("some_count2 = {}", some_count2);

        let (input, _) = if some_count2 == 0 {
            let (input, skeleton_count) = le_u32_as_usize(input)?;
            log::info!("skeleton_count = {}", skeleton_count);
            let (input, _skeletons) =
                many_m_n(skeleton_count, skeleton_count, |input| skeleton(input, version))(input)?;

            let (input, mesh_frag_count) = le_u32_as_usize(input)?;
            log::info!("mesh_frag_count = {}", mesh_frag_count);

            (input, ())
        } else {
            (input, ())
        };

        Ok((input, (name, a_lot_of_vecs, some_count)))
    }

    pub fn print_mesh_info(input: &[u8]) -> IResult<&[u8], ()> {
        let (input, magic) = le_u32(input)?;
        log::info!("magic = 0x{:x}", magic);
        assert_eq!(0x50535943, magic);
        let (input, version) = le_u32(input)?;
        log::info!("version = 0x{:x}", version);
        let (input, scene_flags) = le_u32(input)?;
        log::info!("scene_flags = 0x{:x}", scene_flags);
        let (input, count) = le_u32_as_usize(input)?;
        log::info!("count = {}", count);

        let (input, names) = many_m_n(count, count, name)(input)?;
        log::info!("names = {:#?}", names);

        let (input, some_string) = le_u32_length_c_string(input)?;
        log::info!("some_string = {}", some_string);

        let (input, some_vec3_0) = many_m_n(3, 3, le_f32)(input)?;
        log::info!("some_vec3_0 = {:?}", some_vec3_0);
        let (input, some_vec3_1) = many_m_n(3, 3, le_f32)(input)?;
        log::info!("some_vec3_1 = {:?}", some_vec3_1);

        let (input, _mesh_magic) = if version > 0x12e {
            let (input, mesh_magic) = le_u32(input)?;
            log::info!("mesh_magic = 0x{:x}", mesh_magic);
            assert_eq!(0x4d455348, mesh_magic);
            (input, Some(mesh_magic))
        } else {
            (input, None)
        };

        let (input, mesh_count) = le_u32_as_usize(input)?;
        log::info!("mesh_count = {}", mesh_count);

        let (input, _mesh) = mesh(input, version)?;
        //let (input, meshes) = many_m_n(mesh_count, mesh_count, mesh)(input)?;

        log::info!(
            "Next {} bytes = {:x?}",
            input.len().min(10),
            &input[..input.len().min(10)]
        );

        Ok((input, ()))
    }
}

fn main() -> Result<(), BoxError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let opts: Opts = Opts::parse();

    match opts.subcommand {
        SubCommand::Decompress { input, output } => {
            let mut output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push(".");
                path.push("output");
                path
            });

            std::fs::create_dir_all(&output)?;
            output.push(input.file_name().unwrap());

            let data = read_file(&input)?;

            let (_, data) = decompress(&data).map_err::<BoxError, _>(|_err| "Unable to decompress".into())?;
            let mut file = BufWriter::new(File::create(output)?);
            file.write_all(&data)?;
        }
        SubCommand::ExtractPkg { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push("output");
                path
            });

            std::fs::create_dir_all(&output)?;

            let data = read_file(&input)?;
            let zpkg = Zpkg::from_slice(&data)?;

            for ZpkgFile { path, data } in zpkg.files {
                let path = match path {
                    _ if path.starts_with('/') => &path[1..path.len() - 1],
                    _ => &path,
                };
                write_file(&output.join(path), &data)?;
            }
        }
        SubCommand::CreatePpf { input: _, output: _ } => todo!(),
        SubCommand::ExtractPpf { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push("output");
                path
            });

            std::fs::create_dir_all(&output)?;

            let data = read_file(&input)?;
            let ppf = Ppf::from_slice(&data)?;

            for (id, game_textures) in ppf
                .languages
                .into_iter()
                .chain(vec![(DEFAULT_LANGUAGE, ppf.game_textures)])
            {
                let output = if id == DEFAULT_LANGUAGE {
                    output.clone()
                } else {
                    let file_stem = output.file_stem().and_then(OsStr::to_str).unwrap();
                    let file_ext = output.file_stem().and_then(OsStr::to_str).unwrap();
                    output.with_file_name(format!("{}_{}{}", file_stem, id.to_string().to_lowercase(), file_ext))
                };

                for (index, game_texture) in game_textures.into_iter().enumerate() {
                    let path = match game_texture.path {
                        Some(path) => path.replace("\\", "/"),
                        None => format!("texture_{}.dds", index),
                    };
                    let output = output.join(path);

                    let texture_count = game_texture.textures.len();
                    for (index, texture) in game_texture.textures.into_iter().enumerate() {
                        let output = if texture_count > 1 {
                            let file_stem = output.file_stem().and_then(OsStr::to_str).unwrap();
                            let file_ext = output.extension().and_then(OsStr::to_str).unwrap();
                            output.with_file_name(format!("{}_{:02}.{}", file_stem, index, file_ext))
                        } else {
                            output.clone()
                        };

                        let mut buffer = Vec::with_capacity(4 + size_of::<dds::Header>() + texture.texture.len());
                        buffer.extend_from_slice(&dds::MAGIC.to_le_bytes());
                        buffer.extend_from_slice(&texture.dds_header()?);
                        if let Some(palette) = texture.palette {
                            for item in palette {
                                let mut bytes = item.to_le_bytes();
                                bytes.swap(1, 3);
                                buffer.extend_from_slice(&bytes);
                            }
                        }
                        buffer.extend_from_slice(texture.texture);

                        write_file(&output, &buffer)?;
                    }
                }
            }

            for (path, data) in ppf.meshes {
                write_file(&output.join(path.replace("\\", "/")), data)?;
            }

            for (path, data) in ppf.variables {
                write_file(
                    &output.join(format!("scripts/{}.lua", path.replace("\\", "/").replace(".", "/"))),
                    data,
                )?;
            }

            for (index, Script { path, data }) in ppf.scripts.into_iter().enumerate() {
                let path = match path {
                    Some(path) => path.replace("\\", "/"),
                    None => format!("script_{}.lua", index),
                };
                write_file(&output.join(path), data)?;
            }

            log::info!(
                "Domain first {} bytes = {:02x?}",
                ppf.domain.len().min(10),
                &ppf.domain[..ppf.domain.len().min(10)]
            );
            if !ppf.domain.is_empty() {
                // FIXME: There are multiple plb files here, we need to parse them out. Work started in next SubCommand. Might be worth looking for magic numbers rather than parsing.
                write_file(
                    &output.join(format!(
                        "levels/{}.plb",
                        input.file_stem().and_then(OsStr::to_str).unwrap()
                    )),
                    ppf.domain,
                )?;
            }
        }
        SubCommand::MeshInfo { input } => {
            let data = read_file(&input)?;
            plb::print_mesh_info(&data).map_err::<BoxError, _>(|_err| "Unable to.".into())?;
        }
    }

    Ok(())
}
