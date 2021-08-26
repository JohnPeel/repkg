use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use clap::{AppSettings, Clap};

use pkg::{Zpkg, ZpkgFile};
use ppf::{Ppf, Script, Texture, TextureFormat, TextureType};

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
    decoder
        .read_to_end(&mut buffer)
        .expect("Unable to decode zlib.");
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

        header.pixel_format = match self.format {
            TextureFormat::A8R8G8B8 => dds::PixelFormat::A8R8G8B8,
            TextureFormat::R8G8B8 => dds::PixelFormat::R8G8B8, // FIXME: OpenGL types point to X8R8G8B8, but LoadTextureFromDDSStream points to R8G8B8
            TextureFormat::A4R4G4B4 => dds::PixelFormat::A4R4G4B4,
            TextureFormat::A1R5G5B5 => dds::PixelFormat::A1R5G5B5,
            TextureFormat::X1R5G5B5 => dds::PixelFormat::X1R5G5B5,
            TextureFormat::R5G6B5 => dds::PixelFormat::R5G6B5,
            TextureFormat::A8 => dds::PixelFormat::A8,
            TextureFormat::L8 => dds::PixelFormat::L8, // FIXME: LoadTextureFromDDSStream loads this from A8 dds header.
            TextureFormat::AL8 => unimplemented!(),    // FIXME: Possibly A8L8_ALT or A4L4.
            TextureFormat::DXT1 => dds::PixelFormat::DXT1,
            TextureFormat::DXT3 => dds::PixelFormat::DXT3,
            TextureFormat::DXT5 => dds::PixelFormat::DXT5,
            TextureFormat::V8U8 => dds::PixelFormat::V8U8,
            TextureFormat::V16U16 => dds::PixelFormat::V16U16,
            TextureFormat::PAL8 => (dds::PAL8, 8, 0, 0, 0, 0).into(),
        };

        Ok(bincode::serialize(&header)?)
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

            let data = {
                let file = File::open(input)?;
                let metadata = file.metadata()?;
                let mut reader = BufReader::new(file);

                let mut buffer = Vec::with_capacity(metadata.len() as usize);
                reader.read_to_end(&mut buffer)?;
                buffer
            };

            let (_, data) =
                decompress(&data).map_err::<BoxError, _>(|_err| "Unable to decompress".into())?;
            let mut file = BufWriter::new(File::create(output)?);
            file.write_all(&data)?;
        }
        SubCommand::ExtractPkg { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push(".");
                path.push("output");
                path
            });

            std::fs::create_dir_all(&output)?;

            let data = {
                let file = File::open(input)?;
                let metadata = file.metadata()?;
                let mut reader = BufReader::new(file);

                let mut buffer = Vec::with_capacity(metadata.len() as usize);
                reader.read_to_end(&mut buffer)?;
                buffer
            };

            let zpkg = Zpkg::from_slice(&data)?;

            for ZpkgFile { path, data } in zpkg.files {
                let mut output = output.clone();
                if path.starts_with('/') {
                    output.push(&path[1..path.len() - 1]);
                } else {
                    output.push(&path);
                }

                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent).map_err::<BoxError, _>(|_err| {
                        format!("Unable to create directories for \"{}\"", path).into()
                    })?;
                }

                let mut file = File::create(output)?;
                file.write_all(&data)?;
            }
        }
        SubCommand::ExtractPpf { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut path = PathBuf::new();
                path.push(".");
                path.push("output");
                path.push(input.file_name().unwrap());
                path
            });

            log::info!("input = {:?}", input);
            log::info!("output = {:?}", output);

            std::fs::create_dir_all(&output)?;

            let data = {
                let file = File::open(input)?;
                let metadata = file.metadata()?;
                let mut reader = BufReader::new(file);

                let mut buffer = Vec::with_capacity(metadata.len() as usize);
                reader.read_to_end(&mut buffer)?;
                buffer
            };

            let ppf = Ppf::from_slice(&data)?;

            for (index, game_texture) in ppf.game_textures.into_iter().enumerate() {
                let mut path = output.clone();
                if let Some(texture_path) = game_texture.path {
                    path.push(texture_path.replace("\\", "/"));
                } else {
                    path.push(format!("textures/texture_{}.dds", index));
                }

                let texture_count = game_texture.textures.len();
                if texture_count > 1 {
                    let mut path = path.clone();
                    path.push("metadata.json");

                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    let animation_info =
                        serde_json::to_vec_pretty(&game_texture.animation_info.unwrap())?;

                    let mut writer = BufWriter::new(File::create(path)?);
                    writer.write_all(&animation_info)?;
                }

                for (index, texture) in game_texture.textures.into_iter().enumerate() {
                    let mut path = path.clone();

                    if texture_count > 1 {
                        path.push(format!("frame_{}.dds", index));
                    }

                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    let mut writer = BufWriter::new(File::create(path)?);
                    writer.write_all(&dds::MAGIC.to_le_bytes())?;
                    writer.write_all(&texture.dds_header()?)?;
                    if let Some(palette) = texture.palette {
                        for item in palette {
                            let mut bytes = item.to_le_bytes();
                            bytes.swap(1, 3);
                            writer.write_all(&bytes)?;
                        }
                    }
                    writer.write_all(texture.texture)?;
                }
            }

            for (mesh_path, mesh_data) in ppf.meshes {
                let mut path = output.clone();
                path.push(mesh_path.replace("\\", "/"));

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut file = BufWriter::new(File::create(path)?);
                file.write_all(mesh_data)?;
            }

            for (variable_path, variable_data) in ppf.variables {
                let mut path = output.clone();
                path.push(format!(
                    "variables/{}.lua",
                    variable_path.replace("\\", "/")
                ));

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut file = BufWriter::new(File::create(path)?);
                file.write_all(variable_data)?;
            }

            for (index, Script { path, data }) in ppf.scripts.into_iter().enumerate() {
                let mut output = output.clone();
                if let Some(script_path) = path {
                    output.push(script_path.replace("\\", "/"));
                } else {
                    output.push(format!("scripts/script_{}.lua", index));
                }

                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut file = BufWriter::new(File::create(output)?);
                file.write_all(data)?;
            }

            {
                let mut path = output;
                path.push("domain.bin");

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut file = BufWriter::new(File::create(path)?);
                file.write_all(ppf.domain)?;
            }
        }
    }

    Ok(())
}
