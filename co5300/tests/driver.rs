//! The driver against the `co5300-sim` model.

use co5300::{
    Co5300, Config, EnhancementLevel, Error, FramedSpiInterface, Madctl, PixelFormat, SpiInterface,
    TearingEffect, panel,
};
use co5300_sim::{Co5300Model, FakeDc, FakeDelay, FakeReset, FakeSpi, Protocol};
use core::cell::RefCell;

type Display<'a> = Co5300<SpiInterface<FakeSpi<'a>, FakeDc<'a>>>;

fn display(model: &RefCell<Co5300Model>) -> (Display<'_>, FakeDelay<'_>) {
    let iface = SpiInterface::new(FakeSpi::new(model), FakeDc::new(model));
    let config = Config::new(model.borrow().width(), model.borrow().height());
    (Co5300::new(iface, config), FakeDelay::new(model))
}

/// Resets and initializes the display; checks the datasheet's timing on the way.
fn ready(model: &RefCell<Co5300Model>) -> (Display<'_>, FakeDelay<'_>) {
    let (mut display, mut delay) = display(model);
    co5300::reset(&mut FakeReset::new(model), &mut delay).unwrap();
    display.init(&mut delay, &[]).unwrap();
    (display, delay)
}

fn assert_clean(model: &RefCell<Co5300Model>) {
    let model = model.borrow();
    assert!(model.violations().is_empty(), "{:?}", model.violations());
}

#[test]
fn init_follows_the_datasheet() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (display, _) = ready(&model);
    assert_clean(&model);

    let m = model.borrow();
    assert_eq!(m.commands(), [0xFE, 0xC4, 0x11, 0x3A, 0x51, 0x29]);
    assert!(!m.is_sleeping());
    assert!(m.is_display_on());
    assert!(m.spi_ram_write());
    assert_eq!(u8::from(m.colmod()), 0x55);
    assert_eq!(m.brightness(), 0xFF);
    assert_eq!(display.pixel_format(), PixelFormat::Rgb565);
}

#[test]
fn init_sends_panel_commands_first() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, mut delay) = display(&model);
    co5300::reset(&mut FakeReset::new(&model), &mut delay).unwrap();
    display.init(&mut delay, panel::ESPRESSIF_DEFAULT).unwrap();
    assert_clean(&model);

    let m = model.borrow();
    assert_eq!(
        m.commands(),
        [
            0xFE, 0xC4, 0x35, 0x53, 0x51, 0x63, 0xFE, 0xC4, 0x11, 0x3A, 0x51, 0x29
        ]
    );
    assert_eq!(u8::from(m.ctrl_display()), 0x20);
    assert_eq!(m.hbm(), (0xFF, false));
    assert!(m.tearing_effect().0.is_some());
}

#[test]
fn init_without_reset_breaks_the_timing() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, mut delay) = display(&model);
    display.init(&mut delay, &[]).unwrap();
    assert!(!model.borrow().violations().is_empty());
}

#[test]
fn framed_spi_init_and_draw() {
    let model = RefCell::new(Co5300Model::new(4, 4).with_protocol(Protocol::Framed));
    let mut delay = FakeDelay::new(&model);
    let mut display = Co5300::new(
        FramedSpiInterface::new(FakeSpi::new(&model)),
        Config::new(4, 4),
    );
    co5300::reset(&mut FakeReset::new(&model), &mut delay).unwrap();
    display.init(&mut delay, &[]).unwrap();
    display.set_window(2, 2, 2, 2).unwrap();
    display.write_pixels(&[0xF8, 0x00, 0x07, 0xE0]).unwrap();
    display
        .write_pixels_continue(&[0x00, 0x1F, 0xFF, 0xFF])
        .unwrap();
    assert_clean(&model);

    let m = model.borrow();
    assert!(m.is_display_on());
    assert_eq!(
        [m.pixel(2, 2), m.pixel(3, 2), m.pixel(2, 3), m.pixel(3, 3)],
        [0xFF0000, 0x00FF00, 0x0000FF, 0xFFFFFF]
    );
}

#[test]
fn window_must_be_aligned_and_inside() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, _) = ready(&model);
    assert_eq!(display.set_window(1, 0, 2, 2), Err(Error::InvalidWindow));
    assert_eq!(display.set_window(0, 0, 3, 2), Err(Error::InvalidWindow));
    assert_eq!(display.set_window(6, 6, 4, 2), Err(Error::InvalidWindow));
    display.set_window(2, 4, 4, 2).unwrap();
    let (columns, rows) = model.borrow().window();
    assert_eq!(
        (columns.start, columns.end, rows.start, rows.end),
        (2, 5, 4, 5)
    );
    assert_clean(&model);
}

#[test]
fn offset_shifts_the_window() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let iface = SpiInterface::new(FakeSpi::new(&model), FakeDc::new(&model));
    let mut display = Co5300::new(iface, Config::new(4, 4).with_offset(2, 4));
    display.set_window(0, 0, 4, 4).unwrap();
    let (columns, rows) = model.borrow().window();
    assert_eq!(
        (columns.start, columns.end, rows.start, rows.end),
        (2, 5, 4, 7)
    );
}

#[test]
fn sleep_and_display_control() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, mut delay) = ready(&model);

    display.display_off().unwrap();
    assert!(!model.borrow().is_display_on());
    // SLPIN may follow SLPOUT only after 120 ms.
    delay_ms(&mut delay, 120);
    display.sleep_in(&mut delay).unwrap();
    assert!(model.borrow().is_sleeping());
    display.sleep_out(&mut delay).unwrap();
    display.display_on().unwrap();
    assert!(!model.borrow().is_sleeping());
    assert!(model.borrow().is_display_on());
    assert_clean(&model);
}

#[test]
fn settings_reach_the_chip() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, _) = ready(&model);

    display.set_brightness(0x40).unwrap();
    display.set_hbm_brightness(0x80).unwrap();
    display.set_hbm(true).unwrap();
    display.set_acl(true).unwrap();
    display
        .set_sunlight_enhancement(Some(EnhancementLevel::Medium))
        .unwrap();
    display
        .set_madctl(Madctl::new().with_mx(true).with_bgr(true))
        .unwrap();
    display
        .set_tearing_effect(Some(TearingEffect::VAndHBlank))
        .unwrap();
    display.set_tear_scanline(0x0123).unwrap();
    display.set_idle(true).unwrap();
    display.set_inverted(true).unwrap();
    display.set_pixel_format(PixelFormat::Rgb888).unwrap();
    assert_clean(&model);

    let m = model.borrow();
    assert_eq!(m.brightness(), 0x40);
    assert_eq!(m.hbm(), (0x80, true));
    assert_eq!(u8::from(m.acl()), 0x03);
    assert_eq!(u8::from(m.color_enhance()), 0x05);
    assert_eq!(u8::from(m.madctl()), 0x48);
    let (te, line) = m.tearing_effect();
    assert_eq!((te.map(u8::from), line), (Some(0x01), 0x0123));
    assert!(m.is_idle());
    assert_eq!(u8::from(m.colmod()), 0x77);
    drop(m);

    display.set_tearing_effect(None).unwrap();
    display.set_idle(false).unwrap();
    assert_eq!(model.borrow().tearing_effect().0, None);
    assert!(!model.borrow().is_idle());
}

#[test]
fn deep_standby_and_hardware_reset() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, mut delay) = ready(&model);
    display.enter_deep_standby().unwrap();
    assert!(model.borrow().is_deep_standby());

    co5300::reset(&mut FakeReset::new(&model), &mut delay).unwrap();
    assert!(!model.borrow().is_deep_standby());
    assert!(model.borrow().is_sleeping());
    display.init(&mut delay, &[]).unwrap();
    assert!(model.borrow().is_display_on());
    assert_clean(&model);
}

#[test]
fn software_reset_restores_defaults() {
    let model = RefCell::new(Co5300Model::new(8, 8));
    let (mut display, mut delay) = ready(&model);
    display.software_reset(&mut delay).unwrap();
    assert!(model.borrow().is_sleeping());
    assert_eq!(display.pixel_format(), PixelFormat::Rgb888);
    display.init(&mut delay, &[]).unwrap();
    assert_clean(&model);
}

fn delay_ms(delay: &mut FakeDelay<'_>, ms: u32) {
    use embedded_hal::delay::DelayNs;
    delay.delay_ms(ms);
}

#[cfg(feature = "async")]
mod asynch {
    use super::*;
    use co5300::asynch::Co5300;
    use embassy_futures::block_on;

    #[test]
    fn async_init_and_draw() {
        let model = RefCell::new(Co5300Model::new(4, 4));
        let mut delay = FakeDelay::new(&model);
        let iface = SpiInterface::new(FakeSpi::new(&model), FakeDc::new(&model));
        let mut display = Co5300::new(iface, Config::new(4, 4));

        block_on(async {
            co5300::asynch::reset(&mut FakeReset::new(&model), &mut delay)
                .await
                .unwrap();
            display.init(&mut delay, &[]).await.unwrap();
            display.set_brightness(0x10).await.unwrap();
            display.set_window(0, 0, 2, 2).await.unwrap();
            display.write_pixels(&[0xF8, 0x00]).await.unwrap();
            display.write_pixels_continue(&[0xF8, 0x00]).await.unwrap();
        });
        assert_clean(&model);

        let m = model.borrow();
        assert_eq!(m.brightness(), 0x10);
        assert_eq!(m.pixel(1, 0), 0xFF0000);
    }

    #[test]
    fn async_framed_spi() {
        let model = RefCell::new(Co5300Model::new(4, 4).with_protocol(Protocol::Framed));
        let mut delay = FakeDelay::new(&model);
        let mut display = Co5300::new(
            FramedSpiInterface::new(FakeSpi::new(&model)),
            Config::new(4, 4),
        );

        block_on(async {
            co5300::asynch::reset(&mut FakeReset::new(&model), &mut delay)
                .await
                .unwrap();
            display.init(&mut delay, &[]).await.unwrap();
            display.set_idle(true).await.unwrap();
        });
        assert_clean(&model);
        assert!(model.borrow().is_idle());
    }

    #[test]
    fn async_sleep_brightness_and_window() {
        let model = RefCell::new(Co5300Model::new(8, 8));
        let mut delay = FakeDelay::new(&model);
        let iface = SpiInterface::new(FakeSpi::new(&model), FakeDc::new(&model));
        let mut display = Co5300::new(iface, Config::new(8, 8));

        block_on(async {
            co5300::asynch::reset(&mut FakeReset::new(&model), &mut delay)
                .await
                .unwrap();
            display.init(&mut delay, &[]).await.unwrap();
            // SLPIN may follow SLPOUT only after 120 ms.
            embedded_hal_async::delay::DelayNs::delay_ms(&mut delay, 120).await;
            display.sleep_in(&mut delay).await.unwrap();
            assert!(model.borrow().is_sleeping());
            display.sleep_out(&mut delay).await.unwrap();
            assert!(!model.borrow().is_sleeping());
            display.set_brightness(0x42).await.unwrap();
            assert_eq!(
                display.set_window(1, 0, 2, 2).await,
                Err(Error::InvalidWindow)
            );
            display.set_window(2, 4, 4, 2).await.unwrap();
        });
        assert_clean(&model);

        let m = model.borrow();
        assert_eq!(m.brightness(), 0x42);
        let (columns, rows) = m.window();
        assert_eq!(
            (columns.start, columns.end, rows.start, rows.end),
            (2, 5, 4, 5)
        );
    }
}
