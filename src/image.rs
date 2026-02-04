use crate::Error;
use image::{DynamicImage, Pixel, Rgb, RgbImage};

pub fn encode_webp(img: &DynamicImage) -> Result<Vec<u8>, Error> {
    Ok(webp::Encoder::from_image(img)?.encode(90f32).to_vec())
}

pub fn alpha_to_mask(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let mut mask = RgbImage::new(w, h);

    for (x, y, p) in rgba.enumerate_pixels() {
        let alpha = p[3];
        let v = if alpha == 0 { 0 } else { 255 };
        mask.put_pixel(x, y, Rgb([v, v, v]));
    }

    DynamicImage::ImageRgb8(mask)
}

pub fn background(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let mut mask = RgbImage::new(w, h);

    for (x, y, p) in rgba.enumerate_pixels() {
        let alpha = p[3];
        let rgb = if alpha == 0 {
            Rgb([0, 0, 0])
        } else {
            p.to_rgb()
        };

        mask.put_pixel(x, y, rgb);
    }

    DynamicImage::ImageRgb8(mask)
}
