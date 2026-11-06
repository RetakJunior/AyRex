use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logo = Path::new("src/logo/ayrex.png");
    println!("cargo:rerun-if-changed={}", logo.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let icon_path = output.join("ayrex.ico");
    write_icon(logo, &icon_path)?;

    let icon_path = icon_path.to_string_lossy();
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(&icon_path)
        .set("ProductName", "AyRex")
        .set("FileDescription", "AyRex Screen Recorder")
        .set("InternalName", "AyRex")
        .set("OriginalFilename", "AyRex.exe")
        .compile()?;
    Ok(())
}

fn write_icon(source: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let logo = image::open(source)?.into_rgba8();
    let mut images = Vec::new();
    for size in [16_u32, 32, 48, 256] {
        let resized =
            image::imageops::resize(&logo, size, size, image::imageops::FilterType::Lanczos3);
        images.push(icon_bitmap(&resized)?);
    }

    let header_size = 6_u32 + 16 * images.len() as u32;
    let mut file = fs::File::create(destination)?;
    file.write_all(&0_u16.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&(images.len() as u16).to_le_bytes())?;

    let mut offset = header_size;
    for (size, image) in [16_u8, 32, 48, 0].into_iter().zip(&images) {
        file.write_all(&[size, size, 0, 0])?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&32_u16.to_le_bytes())?;
        file.write_all(&(image.len() as u32).to_le_bytes())?;
        file.write_all(&offset.to_le_bytes())?;
        offset += image.len() as u32;
    }
    for image in images {
        file.write_all(&image)?;
    }
    Ok(())
}

fn icon_bitmap(image: &image::RgbaImage) -> Result<Vec<u8>, io::Error> {
    let width = image.width();
    let height = image.height();
    let pixel_bytes = width * height * 4;
    let mask_stride = width.div_ceil(32) * 4;
    let mask_bytes = mask_stride * height;

    let mut bitmap = Vec::with_capacity((40 + pixel_bytes + mask_bytes) as usize);
    for value in [40_u32, width, height * 2, 1, 32, 0, pixel_bytes, 0, 0, 0, 0] {
        bitmap.write_all(&value.to_le_bytes())?;
    }

    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = image.get_pixel(x, y).0;
            bitmap.write_all(&[pixel[2], pixel[1], pixel[0], pixel[3]])?;
        }
    }
    bitmap.resize(bitmap.len() + mask_bytes as usize, 0);
    Ok(bitmap)
}
