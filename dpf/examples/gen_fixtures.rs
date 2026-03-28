use image::{ImageBuffer, Rgb, Rgba};
use std::fs;

fn main() {
    let fixtures_dir = "test_fixtures";
    fs::create_dir_all(fixtures_dir).expect("Failed to create fixtures dir");

    // 1. sample.png - RGBA 100x100 con gradiente
    let mut png_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(100, 100);
    for (x, y, pixel) in png_img.enumerate_pixels_mut() {
        let r = ((x as f32 / 100.0) * 255.0) as u8;
        let g = ((y as f32 / 100.0) * 255.0) as u8;
        let b = 128u8;
        let a = 255u8;
        *pixel = Rgba([r, g, b, a]);
    }
    png_img
        .save(format!("{}/sample.png", fixtures_dir))
        .expect("Failed to save sample.png");
    println!("✓ Created sample.png (100x100 RGBA)");

    // 2. sample.jpg - JPEG 100x100
    let mut jpg_img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(100, 100);
    for (x, y, pixel) in jpg_img.enumerate_pixels_mut() {
        let r = ((x as f32 / 100.0) * 255.0) as u8;
        let g = ((y as f32 / 100.0) * 255.0) as u8;
        let b = 128u8;
        *pixel = Rgb([r, g, b]);
    }
    jpg_img
        .save(format!("{}/sample.jpg", fixtures_dir))
        .expect("Failed to save sample.jpg");
    println!("✓ Created sample.jpg (100x100 RGB)");

    // 3. sample_transparent.png - PNG con transparencia
    let mut transparent_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(100, 100);
    for (x, y, pixel) in transparent_img.enumerate_pixels_mut() {
        let r = 255u8;
        let g = 0u8;
        let b = 0u8;
        let a = ((x as f32 / 100.0) * 255.0) as u8;
        *pixel = Rgba([r, g, b, a]);
    }
    transparent_img
        .save(format!("{}/sample_transparent.png", fixtures_dir))
        .expect("Failed to save sample_transparent.png");
    println!("✓ Created sample_transparent.png (100x100 with alpha)");

    // 4. sample.svg - SVG vector
    let svg_content = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <rect x="10" y="10" width="80" height="80" fill="FF6B6B"/>
  <circle cx="50" cy="50" r="30" fill="4ECDC4"/>
  <text x="50" y="55" text-anchor="middle" font-size="14" fill="white">TEST</text>
</svg>"##
        .replace("FF6B6B", "#FF6B6B")
        .replace("4ECDC4", "#4ECDC4");
    fs::write(format!("{}/sample.svg", fixtures_dir), svg_content)
        .expect("Failed to save sample.svg");
    println!("✓ Created sample.svg (100x100 vector)");

    // 5. large.png - PNG grande 1000x1000
    let mut large_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(1000, 1000);
    for (x, y, pixel) in large_img.enumerate_pixels_mut() {
        let r = ((x % 256) as u8);
        let g = ((y % 256) as u8);
        let b = 200u8;
        let a = 255u8;
        *pixel = Rgba([r, g, b, a]);
    }
    large_img
        .save(format!("{}/large.png", fixtures_dir))
        .expect("Failed to save large.png");
    println!("✓ Created large.png (1000x1000 RGBA)");

    // 6. solid_red.png - PNG sólido para tests de color
    let red_img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(100, 100, Rgba([255, 0, 0, 255]));
    red_img
        .save(format!("{}/solid_red.png", fixtures_dir))
        .expect("Failed to save solid_red.png");
    println!("✓ Created solid_red.png (solid red)");

    // 7. solid_blue.png
    let blue_img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(100, 100, Rgba([0, 0, 255, 255]));
    blue_img
        .save(format!("{}/solid_blue.png", fixtures_dir))
        .expect("Failed to save solid_blue.png");
    println!("✓ Created solid_blue.png (solid blue)");

    // 8. corrupt/bad.png - Archivo PNG corrupto
    fs::create_dir_all(format!("{}/corrupt", fixtures_dir)).expect("Failed to create corrupt dir");
    fs::write(
        format!("{}/corrupt/bad.png", fixtures_dir),
        b"not a valid png file",
    )
    .expect("Failed to create corrupt file");
    println!("✓ Created corrupt/bad.png (intentionally corrupt)");

    println!("\nAll fixtures generated successfully!");
}
