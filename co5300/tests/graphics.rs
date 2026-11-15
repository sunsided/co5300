//! The `embedded-graphics` target against the `co5300-sim` model.
#![cfg(feature = "embedded-graphics")]

use co5300::{Co5300, Config, Error, PixelFormat, SpiInterface};
use co5300_sim::{Co5300Model, FakeDc, FakeDelay, FakeReset, FakeSpi};
use core::cell::RefCell;
use embedded_graphics_core::Pixel;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::geometry::{Point, Size};
use embedded_graphics_core::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics_core::primitives::Rectangle;

const W: u16 = 8;
const H: u16 = 6;

type Display<'a> = Co5300<SpiInterface<FakeSpi<'a>, FakeDc<'a>>>;

fn ready(model: &RefCell<Co5300Model>) -> Display<'_> {
    let mut delay = FakeDelay::new(model);
    let iface = SpiInterface::new(FakeSpi::new(model), FakeDc::new(model));
    let mut display = Co5300::new(iface, Config::new(W, H));
    co5300::reset(&mut FakeReset::new(model), &mut delay).unwrap();
    display.init(&mut delay, &[]).unwrap();
    display
}

/// The frame memory as characters: `.` black, `R`, `G`, `B`, `W`, `?` other.
fn picture(model: &RefCell<Co5300Model>) -> Vec<String> {
    let m = model.borrow();
    assert!(m.violations().is_empty(), "{:?}", m.violations());
    (0..H)
        .map(|y| {
            (0..W)
                .map(|x| match m.pixel(x, y) {
                    0x000000 => '.',
                    0xFF0000 => 'R',
                    0x00FF00 => 'G',
                    0x0000FF => 'B',
                    0xFFFFFF => 'W',
                    _ => '?',
                })
                .collect()
        })
        .collect()
}

fn rect(x: i32, y: i32, w: u32, h: u32) -> Rectangle {
    Rectangle::new(Point::new(x, y), Size::new(w, h))
}

#[test]
fn aligned_fill_is_exact() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    display.fill_solid(&rect(2, 2, 4, 2), Rgb565::RED).unwrap();
    assert_eq!(
        picture(&model),
        [
            "........", "........", "..RRRR..", "..RRRR..", "........", "........"
        ]
    );
}

#[test]
fn unaligned_fill_widens_to_the_grid() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    display
        .fill_solid(&rect(3, 1, 2, 1), Rgb565::GREEN)
        .unwrap();
    assert_eq!(
        picture(&model),
        [
            "..GGGG..", "..GGGG..", "........", "........", "........", "........"
        ]
    );
}

#[test]
fn contiguous_fill_is_exact_when_aligned() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    let colors = [
        Rgb565::RED,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::WHITE,
        Rgb565::BLUE,
        Rgb565::WHITE,
        Rgb565::RED,
        Rgb565::GREEN,
    ];
    display.fill_contiguous(&rect(0, 0, 4, 2), colors).unwrap();
    assert_eq!(
        picture(&model),
        [
            "RGBW....", "BWRG....", "........", "........", "........", "........"
        ]
    );
}

#[test]
fn contiguous_fill_replicates_edges_when_unaligned() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    // A 3 x 3 area at (1, 1): widened to columns 0..=3 and rows 0..=3.
    let colors = [
        Rgb565::RED,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::GREEN,
        Rgb565::WHITE,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::GREEN,
        Rgb565::RED,
    ];
    display.fill_contiguous(&rect(1, 1, 3, 3), colors).unwrap();
    assert_eq!(
        picture(&model),
        [
            "RRGB....", "RRGB....", "GGWG....", "BBGR....", "........", "........"
        ]
    );
}

#[test]
fn clipping_skips_hidden_colors() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    // 4 x 4 at (-2, -2): only the bottom-right 2 x 2 is visible.
    let colors = (0..16).map(|i| {
        if i % 4 >= 2 && i >= 8 {
            Rgb565::BLUE
        } else {
            Rgb565::RED
        }
    });
    display
        .fill_contiguous(&rect(-2, -2, 4, 4), colors)
        .unwrap();
    assert_eq!(
        picture(&model),
        [
            "BB......", "BB......", "........", "........", "........", "........"
        ]
    );

    display
        .fill_solid(&rect(20, 20, 4, 4), Rgb565::RED)
        .unwrap();
    assert_eq!(picture(&model)[0], "BB......");
}

#[test]
fn single_pixels_become_2x2_blocks() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    display
        .draw_iter([Pixel(Point::new(5, 3), Rgb565::WHITE)])
        .unwrap();
    assert_eq!(
        picture(&model),
        [
            "........", "........", "....WW..", "....WW..", "........", "........"
        ]
    );
}

#[test]
fn clear_fills_the_display() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    display.clear(Rgb565::BLUE).unwrap();
    assert!(picture(&model).iter().all(|row| row == "BBBBBBBB"));
    assert_eq!(model.borrow().visible_pixel(7, 5), 0x0000FF);
}

#[test]
fn drawing_needs_rgb565() {
    let model = RefCell::new(Co5300Model::new(W, H));
    let mut display = ready(&model);
    display.set_pixel_format(PixelFormat::Rgb888).unwrap();
    assert_eq!(display.clear(Rgb565::BLUE), Err(Error::UnsupportedFormat));
}
