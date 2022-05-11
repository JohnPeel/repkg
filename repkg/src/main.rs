use std::{
    ffi::OsStr,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use clap::Parser;

use binrw::{BinRead, BinWrite};

use dds::PixelFormat;
use pkg::{Zpkg, ZpkgFile};
use ppf::{Ppf, Texture, TextureFormat, TextureType};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Parser)]
#[clap(author, version, about = None, long_about = None)]
struct Opts {
    #[clap(short = 'v', long)]
    verbose: bool,
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    Info {
        #[clap(parse(from_os_str))]
        input: PathBuf,
    },
    Extract {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    Split {
        #[clap(parse(from_os_str))]
        input: PathBuf,
        #[clap(short = 'o', long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
}

trait DdsHeader {
    fn dds_header(&self) -> Result<Vec<u8>, BoxError>;
}

impl DdsHeader for Texture {
    fn dds_header(&self) -> Result<Vec<u8>, BoxError> {
        let mut header: dds::Header = dds::Header {
            height: self.height as u32,
            width: self.width as u32,
            depth: 1,
            mip_map_count: self.mipmaps as u32,
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

        if self.mipmaps > 1 {
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

fn main() -> Result<(), BoxError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let opts: Opts = Opts::parse();

    match opts.subcommand {
        SubCommand::Info { input } => {
            log::info!("input = {:?}", input);

            match input.extension() {
                Some(ext) if ext == "pkg" => {
                    let data = read_file(&input)?;
                    let zpkg = Zpkg::from_slice(&data)?;
                    log::info!("{:#?}", zpkg);
                }
                Some(ext) if ext == "ppf" => {
                    let file = File::open(&input)?;
                    let mut reader = BufReader::new(file);
                    let ppf = Ppf::read(&mut reader)?;
                    log::info!("{:#?}", ppf);
                }
                _ => unimplemented!(),
            }
        }
        SubCommand::Extract { input, output } => {
            log::info!("input = {:?}", input);
            let output = output.unwrap_or_else(|| {
                input
                    .parent()
                    .and_then(|x| x.parent())
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf()
            });
            log::info!("output = {:?}", output);

            match input.extension() {
                Some(ext) if ext == "pkg" => {
                    let data = read_file(&input)?;
                    let zpkg = Zpkg::from_slice(&data)?;

                    for ZpkgFile { path, data } in zpkg.files {
                        let path = match path {
                            _ if path.starts_with('/') => &path[1..path.len()],
                            _ => &path,
                        };
                        write_file(&output.join(path), &data)?;
                    }
                }
                Some(ext) if ext == "tpf" => todo!(),
                _ => unimplemented!(),
            }
        }
        SubCommand::Split { input, output } => {
            log::info!("input = {:?}", input);
            let output = output.unwrap_or_else(|| {
                input
                    .parent()
                    .and_then(|x| x.parent())
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf()
            });
            log::info!("output = {:?}", output);

            match input.extension() {
                Some(ext) if ext == "ppf" => {
                    let level_name = input.file_stem().and_then(OsStr::to_str).unwrap();

                    let file = File::open(&input)?;
                    let mut reader = BufReader::new(file);
                    let ppf = Ppf::read(&mut reader)?;

                    for ext in ["tpf", "mpf", "lpf", "plb"] {
                        let output = match ext {
                            "tpf" => output.join("pcpackfiles"),
                            "mpf" => output.join("packfiles"),
                            "lpf" => output.join("scripts").join("packfiles"),
                            "plb" => output.join("levels"),
                            _ => unimplemented!(),
                        };

                        std::fs::create_dir_all(&output)?;

                        let output = output.join(format!("{}.{}", level_name, ext));
                        log::info!("writing {:?}", output);

                        let file = File::create(output)?;
                        let mut writer = BufWriter::new(file);
                        match ext {
                            "tpf" => ppf.textures.write_to(&mut writer)?,
                            "mpf" => ppf.meshes.write_to(&mut writer)?,
                            "lpf" => ppf.scripts.write_to(&mut writer)?,
                            "plb" => ppf.level.write_to(&mut writer)?,
                            _ => unimplemented!(),
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }
    }

    Ok(())
}
