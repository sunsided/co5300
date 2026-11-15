//! [`fake_bus`] adapters and fake pins for the [`Co5300Model`].

use crate::Co5300Model;
use core::cell::RefCell;
use core::convert::Infallible;
use fake_bus::embedded_hal::digital::{ErrorType, OutputPin};
use fake_bus::{Clock, SpiTarget};

/// An SPI device (bus plus `CSX`) with a [`Co5300Model`] attached.
pub type FakeSpi<'a> = fake_bus::FakeSpi<'a, Co5300Model>;

/// A delay that advances the [`Co5300Model`]'s time instead of waiting.
pub type FakeDelay<'a> = fake_bus::FakeDelay<'a, Co5300Model>;

impl SpiTarget for Co5300Model {
    fn select(&mut self) {
        self.spi_select();
    }

    fn deselect(&mut self) {
        self.spi_deselect();
    }

    fn transfer(&mut self, mosi: u8) -> u8 {
        self.spi_byte(mosi);
        0
    }
}

impl Clock for Co5300Model {
    fn advance_ns(&mut self, ns: u64) {
        Co5300Model::advance_ns(self, ns);
    }
}

/// Declares a fake output pin that forwards its level to the model.
macro_rules! fake_pin {
    ($(#[$meta:meta])* $name:ident, $setter:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'a> {
            model: &'a RefCell<Co5300Model>,
        }

        impl<'a> $name<'a> {
            /// Creates a pin connected to the model.
            pub fn new(model: &'a RefCell<Co5300Model>) -> Self {
                Self { model }
            }
        }

        impl ErrorType for $name<'_> {
            type Error = Infallible;
        }

        impl OutputPin for $name<'_> {
            fn set_low(&mut self) -> Result<(), Self::Error> {
                self.model.borrow_mut().$setter(false);
                Ok(())
            }

            fn set_high(&mut self) -> Result<(), Self::Error> {
                self.model.borrow_mut().$setter(true);
                Ok(())
            }
        }
    };
}

fake_pin!(
    /// The `D/CX` pin of a [`Co5300Model`].
    FakeDc,
    set_dc_pin
);

fake_pin!(
    /// The active-low `RESX` pin of a [`Co5300Model`].
    FakeReset,
    set_reset_pin
);

#[cfg(test)]
mod tests {
    use super::*;
    use fake_bus::embedded_hal::delay::DelayNs;
    use fake_bus::embedded_hal::spi::SpiDevice;

    #[test]
    fn dc_pin_protocol() {
        let model = RefCell::new(Co5300Model::new(4, 4));
        let mut spi = FakeSpi::new(&model);
        let mut dc = FakeDc::new(&model);
        let mut delay = FakeDelay::new(&model);

        delay.delay_ms(120);
        dc.set_low().unwrap();
        spi.write(&[0x11]).unwrap();
        delay.delay_ms(60);
        spi.write(&[0x3A]).unwrap();
        dc.set_high().unwrap();
        spi.write(&[0x55]).unwrap();
        dc.set_low().unwrap();
        spi.write(&[0x29]).unwrap();

        let model = model.borrow();
        assert!(model.is_display_on());
        assert_eq!(u8::from(model.colmod()), 0x55);
        assert!(model.violations().is_empty(), "{:?}", model.violations());
    }

    #[test]
    fn reset_pin_and_delay() {
        let model = RefCell::new(Co5300Model::new(4, 4));
        let mut rst = FakeReset::new(&model);
        let mut delay = FakeDelay::new(&model);
        model.borrow_mut().execute(0x29, &[]);
        rst.set_low().unwrap();
        delay.delay_us(10);
        rst.set_high().unwrap();
        assert!(!model.borrow().is_display_on());
    }
}
