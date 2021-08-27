use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    mem::size_of,
    path::{Path, PathBuf},
};

use clap::{AppSettings, Clap};

use dds::PixelFormat;
use pkg::{Zpkg, ZpkgFile};
use ppf::{AnimationInfo, Ppf, Script, Texture, TextureFormat, TextureType};

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
        let mut header: dds::Header = dds::Header::default();

        header.height = self.height as u32;
        header.width = self.width as u32;
        header.mip_map_count = self.mipmap_levels as u32;
        header.depth = 1;

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

        header.pixel_format = texture_format_to_pixel_format(self.format);

        Ok(bincode::serialize(&header)?)
    }
}

fn texture_format_to_pixel_format(format: TextureFormat) -> PixelFormat {
    match format {
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
        TextureFormat::PAL8 => PixelFormat::from_tuple((dds::PAL8, 0, 0, 0, 0, 0)),
    }
}

fn pixel_format_to_texture_format(format: PixelFormat) -> TextureFormat {
    if format.flags == dds::PAL8 {
        log::info!("{:x?}", format);
    }

    match format {
        PixelFormat::A8R8G8B8 => TextureFormat::A8R8G8B8,
        PixelFormat::R8G8B8 => TextureFormat::R8G8B8,
        PixelFormat::A4R4G4B4 => TextureFormat::A4R4G4B4,
        PixelFormat::A1R5G5B5 => TextureFormat::A1R5G5B5,
        PixelFormat::X1R5G5B5 => TextureFormat::X1R5G5B5,
        PixelFormat::R5G6B5 => TextureFormat::R5G6B5,
        PixelFormat::A8 => TextureFormat::A8,
        PixelFormat::L8 => TextureFormat::L8,
        //TextureFormat::AL8 => unimplemented!(),
        PixelFormat::DXT1 => TextureFormat::DXT1,
        PixelFormat::DXT3 => TextureFormat::DXT3,
        PixelFormat::DXT5 => TextureFormat::DXT5,
        PixelFormat::V8U8 => TextureFormat::V8U8,
        PixelFormat::V16U16 => TextureFormat::V16U16,
        _ if format.flags == dds::PAL8 => TextureFormat::PAL8,
        _ => unimplemented!("{:x?}", format),
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, BoxError> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let mut reader = BufReader::new(file);

    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), BoxError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut index = 0;
    let mut path = path.to_path_buf();
    while path.exists() {
        let file_name = path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|str| str.to_string())
            .ok_or_else::<BoxError, _>(|| "Unable to convert file name to String.".into())?;
        let file_ext_start = file_name.rfind('.').unwrap();
        path = path.with_file_name(format!(
            "{}.{}{}",
            &file_name[..file_ext_start],
            index,
            &file_name[file_ext_start..]
        ));
        index += 1;
    }

    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(data)?;
    Ok(())
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
        SubCommand::CreatePpf { input, output } => {
            let input = input.canonicalize()?;
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push(input.file_name().unwrap());
                path
            });
            let append_input = |path: &str| -> String {
                input
                    .join(path)
                    .to_str()
                    .expect("Unable to convert to string.")
                    .to_string()
            };
            if !input.is_dir() {
                return Err("Input must be a directory.".into());
            }

            log::info!("input = {:?}", input);
            log::info!("output = {:?}", output);

            let mut writer = BufWriter::new(File::create(&output)?);

            writer.write_all(&[0x50, 0x50, 0x41, 0x4B, 0xFD, 0xFD, 0x01, 0x00])?;

            let textures: Vec<PathBuf> = glob::glob(&append_input("textures/**/*.dds"))?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|path| !path.file_name().unwrap().to_str().unwrap().starts_with("frame_"))
                .collect();

            log::info!("Texture count: {}", textures.len());

            writer.write_all(&(textures.len() as u16).to_le_bytes())?;
            for game_texture in textures {
                let mut buffer: Vec<u8> = vec![0; 12];

                buffer.extend_from_slice(&1u32.to_le_bytes());
                if game_texture.is_dir() {
                    buffer.extend_from_slice(&1u32.to_le_bytes());
                } else {
                    buffer.extend_from_slice(&0u32.to_le_bytes());
                }
                buffer.extend_from_slice(&[0; 20]);

                if let Some(path) = game_texture.strip_prefix(&input.join("textures"))?.to_str() {
                    buffer.extend_from_slice(&(path.len() as u16 + 1).to_le_bytes());
                    buffer.extend_from_slice(path.as_bytes());
                    buffer.push(0);
                }

                let textures = if game_texture.is_dir() {
                    let metadata: AnimationInfo =
                        serde_json::from_reader(BufReader::new(File::open(game_texture.join("metadata.json"))?))?;

                    buffer.extend_from_slice(&(metadata.frame_count as u32).to_le_bytes());
                    buffer.extend_from_slice(&metadata.start_frame.to_le_bytes());
                    buffer.extend_from_slice(&metadata.loop_frame.to_le_bytes());
                    buffer.extend_from_slice(&0f32.to_le_bytes());
                    buffer.extend_from_slice(&metadata.frame_rate.to_le_bytes());
                    buffer.extend_from_slice(&(metadata.play_mode as u32).to_le_bytes());
                    buffer.extend_from_slice(&(metadata.playing as u8).to_le_bytes());
                    buffer.extend_from_slice(&[0xCD, 0xCD, 0xCD]);

                    (0..metadata.frame_count)
                        .map(|frame| game_texture.join(format!("frame_{}.dds", frame)))
                        .collect()
                } else {
                    vec![game_texture]
                };

                for texture in textures {
                    let data = read_file(&texture)?;

                    let (header, data) = data.split_at(size_of::<dds::Header>() + 4);

                    let dds: dds::Header = bincode::deserialize(&header[4..])?;
                    let format = pixel_format_to_texture_format(dds.pixel_format);
                    let type_ = if dds.caps2.contains(dds::CUBEMAP_ALLFACES) {
                        TextureType::Cubemap
                    } else {
                        TextureType::Bitmap
                    };

                    buffer.extend_from_slice(&0u32.to_le_bytes());
                    buffer.extend_from_slice(&(format as u32).to_le_bytes());
                    buffer.extend_from_slice(&(type_ as u32).to_le_bytes());
                    buffer.extend_from_slice(&0u32.to_le_bytes()); // FIXME: Flags?
                    buffer.extend_from_slice(&(dds.width as u32).to_le_bytes());
                    buffer.extend_from_slice(&(dds.height as u32).to_le_bytes());
                    buffer.extend_from_slice(&(dds.mip_map_count as u32).to_le_bytes());
                    buffer.extend_from_slice(&[0; 16]);
                    buffer.extend_from_slice(data);
                }

                writer.write_all(&[0x20, 0x58, 0x54, 0x31])?;
                writer.write_all(&(buffer.len() as u32).to_le_bytes())?;
                writer.write_all(&buffer)?;
            }

            writer.write_all(b"MPAK")?;

            let meshes: Vec<PathBuf> = glob::glob(&append_input("meshes/**/*.plb"))?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            log::info!("Mesh count: {}", meshes.len());

            writer.write_all(&(meshes.len() as u16).to_le_bytes())?;
            for mesh in meshes {
                let data = read_file(&mesh)?;

                if let Some(path) = mesh.strip_prefix(&input.join("meshes"))?.to_str() {
                    writer.write_all(&(path.len() as u16 + 1).to_le_bytes())?;
                    writer.write_all(path.as_bytes())?;
                    writer.write_all(&[0x00])?;
                }

                writer.write_all(&(data.len() as u32).to_le_bytes())?;
                writer.write_all(&data)?;
            }

            writer.write_all(&[0xFC, 0xFC])?;
            writer.write_all(&1u16.to_le_bytes())?;

            let variables: Vec<PathBuf> = glob::glob(&append_input("variables/**/*.lua"))?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            log::info!("Variables count: {}", variables.len());

            writer.write_all(&(variables.len() as u16).to_le_bytes())?;
            for variable in variables {
                let data = read_file(&variable)?;

                if let Some(path) = variable.strip_prefix(&input.join("variables"))?.to_str() {
                    writer.write_all(&(path.len() as u16 - 4 + 1).to_le_bytes())?;
                    writer.write_all((&path[..path.len() - 4]).as_bytes())?;
                    writer.write_all(&[0x00])?;
                }

                writer.write_all(&(data.len() as u32).to_le_bytes())?;
                writer.write_all(&data)?;
            }

            let scripts: Vec<PathBuf> = glob::glob(&append_input("scripts/**/*.lua"))?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            log::info!("Scripts count: {}", scripts.len());

            writer.write_all(&(scripts.len() as u16).to_le_bytes())?;
            for script in scripts {
                let data = read_file(&script)?;

                if let Some(path) = script.strip_prefix(&input.join("scripts"))?.to_str() {
                    writer.write_all(&(path.len() as u16 + 1).to_le_bytes())?;
                    writer.write_all(path.as_bytes())?;
                    writer.write_all(&[0x00])?;
                }

                writer.write_all(&(data.len() as u32).to_le_bytes())?;
                writer.write_all(&data)?;
            }

            let domain = input.join("domain.bin");
            let data = read_file(&domain)?;
            writer.write_all(&data)?;
        }
        SubCommand::ExtractPpf { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push("output");
                path.push(input.file_name().unwrap());
                path
            });

            std::fs::create_dir_all(&output)?;

            let data = read_file(&input)?;
            let ppf = Ppf::from_slice(&data)?;

            for (index, game_texture) in ppf.game_textures.into_iter().enumerate() {
                let path = match game_texture.path {
                    Some(path) => path.replace("\\", "/"),
                    None => format!("texture_{}.dds", index),
                };
                let output = output.join(format!("textures/{}", path));

                let texture_count = game_texture.textures.len();
                if texture_count > 1 {
                    write_file(
                        &output.join("metadata.json"),
                        &serde_json::to_vec_pretty(&game_texture.animation_info.unwrap())?,
                    )?;
                }

                for (index, texture) in game_texture.textures.into_iter().enumerate() {
                    let output = if texture_count > 1 {
                        output.join(format!("frame_{}.dds", index))
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

            for (path, data) in ppf.meshes {
                write_file(&output.join(format!("meshes/{}", path.replace("\\", "/"))), data)?;
            }

            for (path, data) in ppf.variables {
                write_file(&output.join(format!("variables/{}.lua", path.replace("\\", "/"))), data)?;
            }

            for (index, Script { path, data }) in ppf.scripts.into_iter().enumerate() {
                let path = match path {
                    Some(path) => path.replace("\\", "/"),
                    None => format!("script.{}.lua", index),
                };
                write_file(&output.join(path), data)?;
            }

            write_file(&output.join("domain.bin"), ppf.domain)?;
        }
    }

    Ok(())
}
