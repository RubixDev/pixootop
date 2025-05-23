use chrono::{DateTime, Local};
use image::{GenericImage, Rgb, RgbImage};
use pixoo::DISPLAY_SIZE;

use crate::{PROGRESS_STEPS, fonts};

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Context {
    pub cpu: u8,
    pub mem: u8,
    pub gpu: u8,
    pub gpu_mem: u8,
    pub vol: u8,
    pub net_up: u8,
    pub net_down: u8,
    pub time: DateTime<Local>,
}

pub fn create_frame(ctx: Option<Context>, mut time: DateTime<Local>) -> RgbImage {
    let mut img = RgbImage::new(DISPLAY_SIZE, DISPLAY_SIZE);

    if let Some(ctx) = ctx {
        time = ctx.time;
        draw_progress(
            ctx.vol,
            &mut img,
            0,
            [
                Rgb([0xff, 0xaa, 0x00]),
                Rgb([0xbb, 0x88, 0x00]),
                Rgb([0x88, 0x55, 0x00]),
            ],
        );
        draw_progress(
            ctx.mem,
            &mut img,
            1,
            [
                Rgb([0x00, 0xdd, 0x00]),
                Rgb([0x00, 0x99, 0x00]),
                Rgb([0x00, 0x55, 0x00]),
            ],
        );
        draw_progress(
            ctx.cpu,
            &mut img,
            2,
            [
                Rgb([0x00, 0x00, 0xdd]),
                Rgb([0x00, 0x00, 0x99]),
                Rgb([0x00, 0x00, 0x55]),
            ],
        );
        draw_progress(
            ctx.gpu,
            &mut img,
            3,
            [
                Rgb([0xff, 0x77, 0x00]),
                Rgb([0xbb, 0x55, 0x00]),
                Rgb([0x88, 0x33, 0x00]),
            ],
        );
        draw_progress(
            ctx.gpu_mem,
            &mut img,
            4,
            [
                Rgb([0xdd, 0x00, 0x00]),
                Rgb([0x99, 0x00, 0x00]),
                Rgb([0x55, 0x00, 0x00]),
            ],
        );
        draw_progress(
            ctx.net_up,
            &mut img,
            5,
            [
                Rgb([0x00, 0x99, 0xff]),
                Rgb([0x00, 0x77, 0xbb]),
                Rgb([0x00, 0x55, 0x88]),
            ],
        );
        draw_progress(
            ctx.net_down,
            &mut img,
            6,
            [
                Rgb([0x99, 0x00, 0xff]),
                Rgb([0x66, 0x00, 0x99]),
                Rgb([0x44, 0x00, 0x66]),
            ],
        );
    }

    write_string(
        &time.format("%l:%M").to_string(),
        &mut img,
        (3, 7),
        &[Rgb([0xff, 0x00, 0xff]), Rgb([0xff, 0x00, 0x99])],
        &fonts::FONT_3X5,
        false,
    );
    write_string(
        &time.format("%S").to_string(),
        &mut img,
        (10, 12),
        &[Rgb([0x88, 0x00, 0x88]), Rgb([0x88, 0x00, 0x55])],
        &fonts::FONT_3X4,
        false,
    );

    img
}

fn draw_progress<I: GenericImage>(progress: u8, img: &mut I, y: u32, colors: [I::Pixel; 3]) {
    let full = (progress / PROGRESS_STEPS).min(15);
    let rest = match full >= 15 {
        true => 0,
        false => progress % PROGRESS_STEPS,
    };
    for x in 0..=full {
        img.put_pixel(x as u32, y, colors[0]);
    }
    match rest {
        1 => img.put_pixel(full as u32 + 1, y, colors[2]),
        2 => img.put_pixel(full as u32 + 1, y, colors[1]),
        _ => {}
    }
}

fn write_string<I: GenericImage>(
    str: &str,
    img: &mut I,
    (mut x, y): (u32, u32),
    colors: &[I::Pixel],
    font: &phf::Map<char, &[&[bool]]>,
    spaces: bool,
) {
    let mut i = 0;
    for char in str.chars() {
        let Some(&char) = font.get(&char) else {
            continue;
        };

        for (cy, &line) in char.iter().enumerate() {
            for (cx, &px) in line.iter().enumerate() {
                if px {
                    img.put_pixel(x + cx as u32, y + cy as u32, colors[i]);
                }
            }
        }

        x += char[0].len() as u32 + spaces as u32;
        i += 1;
        i %= colors.len();
    }
}
