use anyhow::Result;
use image::{Rgba, RgbaImage, imageops::FilterType};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::{fs, path::Path};

/// Rasterize the code-owned mint N; no proprietary browser assets are copied.
pub fn generate(root: &Path) -> Result<()> {
    let mut image = RgbaImage::new(1024, 1024);
    let points = [
        (240.0, 740.0),
        (430.0, 280.0),
        (594.0, 740.0),
        (784.0, 280.0),
    ];
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let (px, py) = (x as f32, y as f32);
        let dx = (px - 512.0).abs() - 330.0;
        let dy = (py - 512.0).abs() - 330.0;
        let inside = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - 130.0;
        if inside <= 0.0 {
            *pixel = Rgba([27, 32, 35, 255]);
        }
        for segment in points.windows(2) {
            let (ax, ay) = segment[0];
            let (bx, by) = segment[1];
            let t = (((px - ax) * (bx - ax) + (py - ay) * (by - ay))
                / ((bx - ax).powi(2) + (by - ay).powi(2)))
            .clamp(0.0, 1.0);
            if (px - ax - t * (bx - ax)).hypot(py - ay - t * (by - ay)) < 47.0 {
                *pixel = Rgba([140, 219, 182, 255]);
            }
        }
    }
    let output = root.join("chromium/overlay/chrome/app/theme/chromium");
    fs::create_dir_all(&output)?;
    // Internal-page favicons are tinted by Chromium. Their alpha mask must
    // contain just the N, otherwise tinting turns the tile into a solid block.
    let mut favicon = image.clone();
    for pixel in favicon.pixels_mut() {
        if pixel.0 != [140, 219, 182, 255] {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    for scale in [1, 2, 3] {
        let common = root.join(format!(
            "chromium/overlay/chrome/app/theme/default_{}_percent/common",
            scale * 100
        ));
        fs::create_dir_all(&common)?;
        let icon = image::imageops::resize(&favicon, 16 * scale, 16 * scale, FilterType::Lanczos3);
        icon.save(common.join("favicon_ntp.png"))?;
        icon.save(common.join("favicon_settings.png"))?;
    }
    for size in [16, 22, 24, 32, 48, 64, 128, 256] {
        image::imageops::resize(&image, size, size, FilterType::Lanczos3)
            .save(output.join(format!("product_logo_{size}.png")))?;
        // GRIT resolves chrome_scaled_image inside a scale-specific directory.
        // The unscaled directory alone leaves WebUI using upstream logos.
        for scale in [1, 2, 3] {
            let scaled = root.join(format!(
                "chromium/overlay/chrome/app/theme/default_{}_percent/chromium",
                scale * 100
            ));
            fs::create_dir_all(&scaled)?;
            image::imageops::resize(&image, size * scale, size * scale, FilterType::Lanczos3)
                .save(scaled.join(format!("product_logo_{size}.png")))?;
        }
    }
    // Windows 11 accepts PNG-backed ICO frames. Keep all sizes in the same
    // code-owned resource rather than shipping Chromium's blue application
    // icon on that platform.
    let sizes = [16_u32, 24, 32, 48, 64, 128, 256];
    let mut directory = Vec::new();
    directory.extend_from_slice(&0_u16.to_le_bytes());
    directory.extend_from_slice(&1_u16.to_le_bytes());
    directory.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut pixels = Vec::new();
    let mut offset = 6 + 16 * sizes.len() as u32;
    for size in sizes {
        let png = fs::read(output.join(format!("product_logo_{size}.png")))?;
        let length = u32::try_from(png.len())?;
        directory.extend_from_slice(&[size as u8, size as u8, 0, 0]);
        directory.extend_from_slice(&1_u16.to_le_bytes());
        directory.extend_from_slice(&32_u16.to_le_bytes());
        directory.extend_from_slice(&length.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        pixels.extend_from_slice(&png);
        offset += length;
    }
    directory.extend_from_slice(&pixels);
    fs::create_dir_all(output.join("win"))?;
    fs::write(output.join("win/chromium.ico"), directory)?;
    #[cfg(target_os = "macos")]
    {
        let iconset = root.join(".cache/NavZip.iconset");
        fs::create_dir_all(&iconset)?;
        for size in [16, 32, 128, 256, 512] {
            image::imageops::resize(&image, size, size, FilterType::Lanczos3)
                .save(iconset.join(format!("icon_{size}x{size}.png")))?;
            image::imageops::resize(&image, size * 2, size * 2, FilterType::Lanczos3)
                .save(iconset.join(format!("icon_{size}x{size}@2x.png")))?;
        }
        fs::create_dir_all(output.join("mac"))?;
        super::chromium::run(
            Command::new("iconutil")
                .args(["-c", "icns"])
                .arg(&iconset)
                .arg("-o")
                .arg(output.join("mac/app.icns")),
        )?;
    }
    println!("Ícones NavZip gerados no overlay");
    Ok(())
}
