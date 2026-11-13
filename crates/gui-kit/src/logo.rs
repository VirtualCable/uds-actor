/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use eframe::egui::{ColorImage, Context, IconData, TextureHandle, TextureOptions};
use image::ImageFormat;

const LOGO_PNG: &[u8] = include_bytes!("../assets/uds-128.png");

struct Rgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn decode() -> Rgba {
    let decoded = image::load_from_memory_with_format(LOGO_PNG, ImageFormat::Png)
        .expect("embedded UDS logo is a valid PNG")
        .to_rgba8();
    Rgba {
        width: decoded.width(),
        height: decoded.height(),
        pixels: decoded.into_raw(),
    }
}

pub fn texture(ctx: &Context) -> TextureHandle {
    let logo = decode();
    let image = ColorImage::from_rgba_unmultiplied(
        [logo.width as usize, logo.height as usize],
        &logo.pixels,
    );
    ctx.load_texture("uds-logo", image, TextureOptions::LINEAR)
}

pub fn window_icon() -> IconData {
    let logo = decode();
    IconData {
        rgba: logo.pixels,
        width: logo.width,
        height: logo.height,
    }
}
