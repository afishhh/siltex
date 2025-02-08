use std::{fmt::Debug, mem::MaybeUninit, path::PathBuf, process::ExitCode};

use clap::Parser;

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[clap(name = "tex2png")]
    Tex2Png(Tex2Png),
}

#[derive(clap::Parser)]
struct Tex2Png {
    tex_path: PathBuf,
    #[clap(short = 'o', long = "output")]
    output_path: Option<PathBuf>,
}

const MAGIC: [u8; 4] = *b"TEX\n";

#[expect(dead_code)]
struct TexHeader {
    magic: [u8; 4],
    version: u8,
    format: u8,
    mipmaps: u8,
    opaque_bitmap: u8,
    width: i16,
    height: i16,
    scale: i32,
    pixels_offset: i32,
    pixels_size: i32,
    bitmap_offset: i32,
    bitmap_size: i32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum TexFormat {
    Bgra8888 = 0x08,
    Bgra5551 = 0x0A,
    Pvrtc2Rgba = 0x84,
}

impl TexFormat {
    pub fn from_value(value: u8) -> Option<TexFormat> {
        Some(match value {
            0x08 => TexFormat::Bgra8888,
            0x0A => TexFormat::Bgra5551,
            0x84 => TexFormat::Pvrtc2Rgba,
            _ => return None,
        })
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let mut tex = match args.command {
        Command::Tex2Png(ref tex2png) => std::fs::read(&tex2png.tex_path).unwrap(),
    };

    let out_path = match args.command {
        Command::Tex2Png(ref tex2png) => tex2png.output_path.clone().unwrap_or_else(|| {
            if tex2png.tex_path.extension().is_some_and(|e| e == "tex") {
                tex2png
                    .tex_path
                    .strip_prefix(tex2png.tex_path.parent().unwrap())
                    .unwrap()
                    .with_extension("png")
            } else {
                eprintln!("No output path provided and tex path doesn't have .tex extension");
                std::process::exit(1);
            }
        }),
    };

    if tex.len() < 32 {
        eprintln!("File is not a tex file: too short");
        return ExitCode::FAILURE;
    }

    let header = TexHeader {
        magic: tex[..4].try_into().unwrap(),
        version: tex[4],
        format: tex[5],
        mipmaps: tex[6],
        opaque_bitmap: tex[7],
        width: i16::from_be_bytes(tex[8..10].try_into().unwrap()),
        height: i16::from_be_bytes(tex[10..12].try_into().unwrap()),
        scale: i32::from_be_bytes(tex[12..16].try_into().unwrap()),
        pixels_offset: i32::from_be_bytes(tex[16..20].try_into().unwrap()),
        pixels_size: i32::from_be_bytes(tex[20..24].try_into().unwrap()),
        bitmap_offset: i32::from_be_bytes(tex[24..28].try_into().unwrap()),
        bitmap_size: i32::from_be_bytes(tex[28..32].try_into().unwrap()),
    };
    if header.magic != MAGIC {
        eprintln!("File is not a tex file: mismatched magic");
        return ExitCode::FAILURE;
    }

    if header.version != 2 {
        eprintln!("Unsupported tex file version: {}", header.version);
        return ExitCode::FAILURE;
    }

    let Some(format) = TexFormat::from_value(header.format) else {
        eprintln!("Unsupported texture format: 0x{:02X}", header.format);
        return ExitCode::FAILURE;
    };

    let pixels = &mut tex[header.pixels_offset as usize
        ..header.pixels_offset as usize + header.pixels_size as usize];
    let temporary;
    let buffer = match format {
        TexFormat::Bgra8888 => {
            assert_eq!(
                header.width as usize * header.height as usize * 4,
                pixels.len()
            );

            for i in (0..pixels.len()).step_by(4) {
                let pixel: &mut [u8; 4] = (&mut pixels[i..i + 4]).try_into().unwrap();
                *pixel = [pixel[2], pixel[1], pixel[0], pixel[3]];
            }

            &*pixels
        }
        TexFormat::Bgra5551 => {
            eprintln!("warning: Assuming little-endian for BGRA5551 format");

            assert_eq!(
                header.width as usize * header.height as usize * 2,
                pixels.len()
            );

            let mut buffer = vec![
                MaybeUninit::<u8>::uninit();
                header.width as usize * header.height as usize * 4
            ];

            for i in (0..pixels.len()).step_by(2) {
                let pixel: &[u8; 2] = (&pixels[i..i + 2]).try_into().unwrap();
                let out: &mut MaybeUninit<[u8; 4]> = unsafe {
                    std::mem::transmute(
                        std::convert::TryInto::<&mut [MaybeUninit<u8>; 4]>::try_into(
                            &mut buffer[i << 1..(i << 1) + 4],
                        )
                        .unwrap_unchecked(),
                    )
                };

                let pixel_value = u16::from_le_bytes(*pixel) as u32;
                let rgba = (((pixel_value >> 10) & 0x1F) * 0xFF / 0x1F)
                    | ((((pixel_value >> 5) & 0x1F) * 0xFF / 0x1F) << 8)
                    | (((pixel_value & 0x1F) * 0xFF / 0x1F) << 16)
                    | ((pixel_value >> 15) * 0xFF000000);
                // println!("{:016b} -> #{:08X}", pixel_value, rgba);
                out.write(rgba.to_le_bytes());
            }

            temporary = unsafe {
                let init = Vec::from_raw_parts(
                    buffer.as_mut_ptr() as *mut u8,
                    buffer.len(),
                    buffer.capacity(),
                );
                std::mem::forget(buffer);
                init
            };

            &temporary
        }
        _ => {
            eprintln!("Conversion from {format:?} is not implemented yet");
            return ExitCode::FAILURE;
        }
    };

    let mut encoder = png::Encoder::new(
        std::fs::File::create(out_path).unwrap(),
        header.width as u32,
        header.height as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(buffer).unwrap();

    ExitCode::SUCCESS
}
