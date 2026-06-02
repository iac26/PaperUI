//! GT911 capacitive touch over I2C, exposed as a simple point reader.
//!
//! gt911 0.3 `Gt911Blocking<I2C>` is generic over the I2C type and borrows the
//! bus per call (it holds only the 7-bit address + a `PhantomData<I2C>`).
//! `get_touch(&self, &mut I2C) -> Result<Option<Point>, Error<E>>` where `Point`
//! exposes `x: u16` / `y: u16`. `Default` selects the standard 0x5D address.

use esp_hal::i2c::master::I2c;
use esp_hal::Blocking;

pub struct TouchInput<'d> {
    i2c: I2c<'d, Blocking>,
    dev: gt911::Gt911Blocking<I2c<'d, Blocking>>,
}

impl<'d> TouchInput<'d> {
    pub fn new(i2c: I2c<'d, Blocking>) -> Self {
        let dev = gt911::Gt911Blocking::default();
        Self { i2c, dev }
    }

    pub fn read_point(&mut self) -> Option<(u16, u16)> {
        match self.dev.get_touch(&mut self.i2c) {
            Ok(Some(p)) => Some((p.x, p.y)),
            _ => None,
        }
    }
}
