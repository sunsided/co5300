//! Wokwi custom chip wrapping the `co5300-sim` command-level model, with a frame buffer.
//!
//! Wokwi has no Quad-SPI, so the chip listens on plain SPI (mode 0): `CSX`, `SCL`, `SDI`,
//! plus `DCX` in the default 4-wire mode. With the `framed` attribute set to `1`, it decodes
//! the single-line `02h` framing instead and ignores `DCX`. `RESX` is active low with a
//! pull-up. The display shows what the panel would: black while asleep or off.

use co5300_sim::{Co5300Model, Protocol};
use std::cell::RefCell;
use std::ffi::{CString, c_void};
use wokwi_chip_ll::*;

const SPI_BUFFER: usize = 1024;

struct Chip {
    model: Co5300Model,
    spi: SPIDevId,
    cs: PinId,
    dc: PinId,
    framebuffer: BufferId,
    /// The bytes the simulator fills; boxed, so its address stays put while the chip moves.
    buffer: Box<[u8; SPI_BUFFER]>,
    /// The `DCX` level when `CSX` went low; the driver only changes it while `CSX` is high.
    dc_at_select: bool,
    last_ns: f64,
}

thread_local! {
    static CHIP: RefCell<Option<Chip>> = const { RefCell::new(None) };
}

fn with_chip<R>(f: impl FnOnce(&mut Chip) -> R) -> R {
    CHIP.with(|chip| f(chip.borrow_mut().as_mut().expect("chipInit not called")))
}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

fn debug(message: &str) {
    let message = cstr(message);
    // SAFETY: `message` is a valid NUL-terminated string for the duration of the call.
    unsafe { debugPrint(message.as_ptr()) };
}

impl Chip {
    /// Advances the model's time to the simulation time.
    fn sync_time(&mut self) {
        // SAFETY: Plain query of the simulator clock.
        let now = unsafe { getSimNanos() };
        let delta = (now - self.last_ns).max(0.0);
        self.last_ns = now;
        self.model.advance_ns(delta as u64);
    }

    fn start_spi(&mut self) {
        // SAFETY: The buffer is boxed and owned by the chip, which lives for the whole
        // simulation, so the pointer stays valid until the simulator calls `on_spi_done`.
        unsafe { spiStart(self.spi, self.buffer.as_ptr(), SPI_BUFFER as u32) };
    }

    /// Copies the changed part of the panel into the frame buffer as RGBA.
    fn render(&mut self) {
        let dirty = self.model.take_dirty();
        // Nothing here reads the command log or the violations; drop them so memory stays flat.
        self.model.clear_log();
        let Some(region) = dirty else {
            return;
        };

        let width = u32::from(self.model.width());
        let mut row = Vec::with_capacity(4 * usize::from(region.x1 - region.x0 + 1));

        for y in region.y0..=region.y1 {
            row.clear();
            for x in region.x0..=region.x1 {
                let rgb = self.model.visible_pixel(x, y);
                row.extend_from_slice(&[(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 0xFF]);
            }

            let offset = 4 * (u32::from(y) * width + u32::from(region.x0));
            // SAFETY: `row` is valid for `row.len()` bytes; the simulator copies it.
            unsafe { bufferWrite(self.framebuffer, offset, row.as_ptr(), row.len() as u32) };
        }
    }
}

extern "C" fn on_spi_done(_: *mut c_void, buffer: *mut u8, count: u32) {
    with_chip(|chip| {
        chip.sync_time();
        chip.model.set_dc_pin(chip.dc_at_select);

        if count > 0 {
            // SAFETY: The simulator hands back our own buffer with `count` valid bytes.
            let bytes = unsafe { std::slice::from_raw_parts(buffer, count as usize) };
            for &byte in bytes {
                chip.model.spi_byte(byte);
            }
        }

        // SAFETY: The pin ID was obtained from the simulator in `chipInit`.
        if unsafe { pinRead(chip.cs) } == LOW {
            chip.start_spi();
        } else {
            chip.model.spi_deselect();
        }

        chip.render();
    });
}

extern "C" fn on_cs_change(_: *mut c_void, _pin: PinId, value: u32) {
    if value == LOW {
        with_chip(|chip| {
            chip.sync_time();
            // SAFETY: The pin ID was obtained from the simulator in `chipInit`.
            chip.dc_at_select = unsafe { pinRead(chip.dc) } == HIGH;
            chip.model.spi_select();
            chip.start_spi();
        });
    } else {
        let spi = with_chip(|chip| chip.spi);
        // The simulator may call `on_spi_done` from here, so the chip must not be borrowed.
        // SAFETY: The device ID was obtained from the simulator in `chipInit`.
        unsafe { spiStop(spi) };
    }
}

extern "C" fn on_reset_change(_: *mut c_void, _pin: PinId, value: u32) {
    with_chip(|chip| {
        chip.sync_time();
        chip.model.set_reset_pin(value == HIGH);
        chip.render();
    });
}

/// Entry point called by the simulator.
#[unsafe(no_mangle)]
pub extern "C" fn chipInit() {
    // SAFETY: All pointers passed to the simulator are valid for the duration of each
    // call; configs are copied by the simulator. Init functions are only called here.
    unsafe {
        let cs = pinInit(cstr("CSX").as_ptr(), INPUT_PULLUP);
        let dc = pinInit(cstr("DCX").as_ptr(), INPUT);
        let rst = pinInit(cstr("RESX").as_ptr(), INPUT_PULLUP);
        let sck = pinInit(cstr("SCL").as_ptr(), INPUT);
        let mosi = pinInit(cstr("SDI").as_ptr(), INPUT);
        let framed = attrRead(attrInit(cstr("framed").as_ptr(), 0.0)) != 0;

        let (mut width, mut height) = (0u32, 0u32);
        let framebuffer = framebufferInit(&mut width, &mut height);

        let spi = spiInit(&SPIConfig {
            user_data: std::ptr::null(),
            sck,
            mosi,
            miso: NO_PIN,
            mode: 0,
            done: on_spi_done,
        });

        for (pin, callback) in [
            (cs, on_cs_change as extern "C" fn(*mut c_void, PinId, u32)),
            (rst, on_reset_change),
        ] {
            pinWatch(
                pin,
                &WatchConfig {
                    user_data: std::ptr::null(),
                    edge: BOTH,
                    pin_change: callback,
                },
            );
        }

        let protocol = if framed {
            Protocol::Framed
        } else {
            Protocol::DcPin
        };
        let model = Co5300Model::new(width as u16, height as u16).with_protocol(protocol);
        let mut chip = Chip {
            model,
            spi,
            cs,
            dc,
            framebuffer,
            buffer: Box::new([0; SPI_BUFFER]),
            dc_at_select: true,
            last_ns: getSimNanos(),
        };
        chip.model.set_reset_pin(pinRead(rst) == HIGH);
        chip.render();

        CHIP.with(|c| *c.borrow_mut() = Some(chip));

        debug(&format!(
            "co5300: chip initialized, {width}x{height}, {protocol:?}"
        ));
    }
}
