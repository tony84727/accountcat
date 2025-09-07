use std::{
    fs::File,
    io::{Cursor, Write},
    path::PathBuf,
};

use clap::Parser;
use image::{GenericImage, GenericImageView, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Parser)]
struct Args {
    /// output directory, with rsbuild, it should be public/ so that generated assets will be
    /// copied to dist
    #[arg(short, long, default_value = "public/")]
    output_directory: PathBuf,
    /// Original file of app icon
    #[arg(short, long)]
    app_icon: PathBuf,
    #[arg(short, long)]
    build_info_output: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct Icon {
    src: String,
    sizes: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    icon_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct WebManifest {
    short_name: String,
    name: String,
    icons: Vec<Icon>,
    start_url: String,
    display: String,
    theme_color: String,
    background_color: String,
}

#[derive(Serialize, Deserialize)]
struct WebManifestBuild {
    manifest_name: String,
}

const ICON_DIMENSIONS: [u32; 2] = [512, 192];

fn get_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    let mut buf = [0u8; 64];
    let result = base16ct::lower::encode_str(&hash, &mut buf).unwrap();
    String::from(&result[..8])
}

fn main() {
    let Args {
        output_directory,
        app_icon,
        build_info_output,
    } = Args::parse();
    if !output_directory.is_dir() {
        std::fs::create_dir_all(&output_directory).unwrap()
    }

    let icon_source = image::open(app_icon).expect("read icon source image");
    let (origin_x, origin_y) = icon_source.dimensions();
    let margin_precentage = 0.1_f32;
    let mut icon_paths: Vec<String> = Vec::with_capacity(ICON_DIMENSIONS.len());
    for size in ICON_DIMENSIONS.into_iter() {
        let mut output = RgbaImage::new(size, size);
        let margin = (size as f32 * margin_precentage) as u32;
        let scaled = icon_source.resize(
            size - 2 * margin,
            size - 2 * margin,
            image::imageops::FilterType::Lanczos3,
        );
        let scale = (size - 2 * margin) as f32 / origin_x.max(origin_y) as f32;
        let scaled_x = origin_x as f32 * scale;
        let scaled_y = origin_y as f32 * scale;
        output
            .copy_from(
                &scaled,
                ((size as f32 - scaled_x) / 2.0).floor() as u32,
                ((size as f32 - scaled_y) / 2.0).floor() as u32,
            )
            .unwrap();
        let mut buf = Vec::new();
        let mut seekable = Cursor::new(&mut buf);
        output
            .write_to(&mut seekable, image::ImageFormat::Png)
            .unwrap();
        let hash = get_hash(&buf);
        let filename = format!("app-icon-{size}x{size}.{hash}.png");
        let output_path = output_directory.clone().join(&filename);
        let mut output = File::create(&output_path).unwrap();
        icon_paths.push(filename);
        output.write_all(&buf).unwrap();
    }
    let manifest = WebManifest {
        short_name: String::from("Accountcat"),
        name: String::from("Accountcat"),
        icons: icon_paths
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                let size = ICON_DIMENSIONS[i];
                Icon {
                    src: format!("/{p}"),
                    sizes: format!("{size}x{size}"),
                    icon_type: Some(String::from("image/png")),
                }
            })
            .collect(),
        start_url: String::from("/"),
        display: String::from("standalone"),
        theme_color: String::from("#505050"),
        background_color: String::from("#ffffff"),
    };
    let mut buf = Vec::new();
    serde_json::to_writer(&mut buf, &manifest).unwrap();
    let hash = get_hash(&buf);
    let manifest_filename = format!("manifest.{hash}.json");
    let mut manifest_file = File::create(output_directory.join(&manifest_filename)).unwrap();
    manifest_file.write_all(&buf).unwrap();
    let Some(build_info_output) = build_info_output else {
        return;
    };
    let mut build_info_file = File::create(build_info_output).unwrap();
    let build_info = WebManifestBuild {
        manifest_name: manifest_filename,
    };
    serde_json::to_writer(&mut build_info_file, &build_info).unwrap();
}
