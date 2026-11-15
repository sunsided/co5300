//! An `embedded-graphics` [`DrawTarget`] on the blocking driver.
//!
//! Drawing goes straight to the chip's frame memory: every fill sets a window
//! (`CASET`/`RASET`) and streams the pixels with `RAMWR` and `RAMWRC`, one row per transfer.
//! The host keeps no frame buffer. The target needs the RGB565 pixel format, which
//! [`Co5300::init`] selects by default.
//!
//! ## Alignment
//!
//! The chip accepts only windows with an even start and an even length, in both directions
//! (datasheet sections 7.5.21 and 7.5.22). Areas that do not satisfy this are widened by one
//! pixel on the unaligned sides, and the widened border repeats the nearest drawn pixel.
//! Drawing on even coordinates with even sizes is exact; anything else also paints the
//! neighbouring pixels up to the next 2 × 2 boundary. Single pixels from
//! [`draw_iter`](DrawTarget::draw_iter) become 2 × 2 blocks.

use crate::{Co5300, Command, Error, Interface, PixelFormat};
use co5300_registers::AddressRange;
use embedded_graphics_core::Pixel;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::geometry::{Dimensions, OriginDimensions, Size};
use embedded_graphics_core::pixelcolor::raw::{RawData, RawU16};
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::primitives::Rectangle;

/// The most columns the chip drives (480 sources, datasheet section 1), plus the alignment
/// margin.
const MAX_ROW_PIXELS: usize = 480 + 2;

/// A clipped area, in display coordinates, and its aligned window in frame memory.
struct Plan {
    /// The drawn area, clipped to the display (inclusive corners).
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    columns: AddressRange,
    rows: AddressRange,
}

impl Plan {
    /// The number of pixels in an aligned row.
    fn row_pixels(&self) -> usize {
        usize::from(self.columns.len())
    }

    /// The display column of the first aligned column.
    fn first_column(&self, column_offset: u16) -> i32 {
        i32::from(self.columns.start) - i32::from(column_offset)
    }
}

impl<IFACE: Interface> Co5300<IFACE> {
    /// Clips `area` to the display and widens it to the 2-pixel grid of the frame memory.
    fn plan(&self, area: &Rectangle) -> Result<Option<Plan>, Error<IFACE::Error>> {
        if self.format != PixelFormat::Rgb565 {
            return Err(Error::UnsupportedFormat);
        }

        let clipped = area.intersection(&self.bounding_box());
        let Some(bottom_right) = clipped.bottom_right() else {
            return Ok(None);
        };
        let (x0, y0) = (clipped.top_left.x, clipped.top_left.y);
        let (x1, y1) = (bottom_right.x, bottom_right.y);

        // In frame memory coordinates, round the start down and the end up to the grid.
        let column_offset = i32::from(self.config.column_offset);
        let row_offset = i32::from(self.config.row_offset);
        let align = |start: i32, end: i32| -> Option<AddressRange> {
            let start = u16::try_from(start & !1).ok()?;
            let end = u16::try_from(end | 1).ok()?;
            let range = AddressRange::new(start, end);
            range.is_valid().then_some(range)
        };

        let columns = align(x0 + column_offset, x1 + column_offset).ok_or(Error::InvalidWindow)?;
        let rows = align(y0 + row_offset, y1 + row_offset).ok_or(Error::InvalidWindow)?;

        if usize::from(columns.len()) > MAX_ROW_PIXELS {
            return Err(Error::InvalidWindow);
        }

        Ok(Some(Plan {
            x0,
            y0,
            x1,
            y1,
            columns,
            rows,
        }))
    }

    /// Sets the window of a plan, in frame memory coordinates.
    fn set_plan_window(&mut self, plan: &Plan) -> Result<(), Error<IFACE::Error>> {
        self.write_command(Command::CASET.code(), &plan.columns.to_params())?;
        self.write_command(Command::RASET.code(), &plan.rows.to_params())
    }

    /// Sends a row, as the first memory write or a continuation.
    fn send_row(&mut self, row: &[u8], first: &mut bool) -> Result<(), Error<IFACE::Error>> {
        if core::mem::take(first) {
            self.write_pixels(row)
        } else {
            self.write_pixels_continue(row)
        }
    }

    /// How often the row at display line `y` is sent: once, plus once more for each
    /// alignment margin it fills.
    fn repeats(&self, plan: &Plan, y: i32) -> usize {
        let row_offset = i32::from(self.config.row_offset);
        let top_margin = i32::from(plan.rows.start) < plan.y0 + row_offset;
        let bottom_margin = i32::from(plan.rows.end) > plan.y1 + row_offset;
        1 + usize::from(y == plan.y0 && top_margin) + usize::from(y == plan.y1 && bottom_margin)
    }
}

fn to_bytes(color: Rgb565) -> [u8; 2] {
    RawU16::from(color).into_inner().to_be_bytes()
}

impl<IFACE> OriginDimensions for Co5300<IFACE> {
    fn size(&self) -> Size {
        Size::new(self.config.width.into(), self.config.height.into())
    }
}

impl<IFACE: Interface> DrawTarget for Co5300<IFACE> {
    type Color = Rgb565;
    type Error = Error<IFACE::Error>;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.fill_solid(&Rectangle::new(point, Size::new(1, 1)), color)?;
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let Some(plan) = self.plan(area)? else {
            return Ok(());
        };
        self.set_plan_window(&plan)?;

        let mut row = [0u8; 2 * MAX_ROW_PIXELS];
        let row_pixels = plan.row_pixels();
        let first_column = plan.first_column(self.config.column_offset);
        let width = area.size.width as usize;
        let mut colors = colors.into_iter();
        let mut first = true;

        for y in area.rows() {
            if y < plan.y0 || y > plan.y1 {
                colors.by_ref().take(width).for_each(drop);
                continue;
            }

            for (x, color) in area.columns().zip(colors.by_ref().take(width)) {
                if x < plan.x0 || x > plan.x1 {
                    continue;
                }

                let i = (x - first_column) as usize;
                row[2 * i..2 * i + 2].copy_from_slice(&to_bytes(color));
            }

            // Repeat the edge pixels into the alignment margins.
            let left = (plan.x0 - first_column) as usize;
            let right = (plan.x1 - first_column) as usize;
            row.copy_within(2 * left..2 * left + 2, 0);
            row.copy_within(2 * right..2 * right + 2, 2 * (row_pixels - 1));

            for _ in 0..self.repeats(&plan, y) {
                self.send_row(&row[..2 * row_pixels], &mut first)?;
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let Some(plan) = self.plan(area)? else {
            return Ok(());
        };
        self.set_plan_window(&plan)?;

        let mut row = [0u8; 2 * MAX_ROW_PIXELS];
        let bytes = to_bytes(color);
        let row_pixels = plan.row_pixels();
        for pixel in row[..2 * row_pixels].chunks_exact_mut(2) {
            pixel.copy_from_slice(&bytes);
        }

        let mut first = true;
        for _ in 0..plan.rows.len() {
            self.send_row(&row[..2 * row_pixels], &mut first)?;
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_solid(&self.bounding_box(), color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_is_msb_first() {
        assert_eq!(to_bytes(Rgb565::new(31, 0, 0)), [0xF8, 0x00]);
        assert_eq!(to_bytes(Rgb565::new(0, 63, 0)), [0x07, 0xE0]);
        assert_eq!(to_bytes(Rgb565::new(0, 0, 31)), [0x00, 0x1F]);
    }
}
