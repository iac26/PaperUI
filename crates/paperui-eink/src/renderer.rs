//! EinkRenderer: implements paperui-core's Renderer over the IT8951, mapping the
//! engine's UpdateHint to an EPD waveform mode and — crucially — waking the panel
//! before a draw and sleeping it after (idle current ~0).

use crate::canvas::Gray4Canvas;
use crate::it8951::It8951;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;
use paperui_core::{Rect, Renderer, UpdateHint};

fn mode_for(hint: UpdateHint) -> u16 {
    match hint {
        UpdateHint::None | UpdateHint::Mono => 1,
        UpdateHint::Fast => 1,
        UpdateHint::Text => 3,
        UpdateHint::Quality => 2,
    }
}

fn pack_region(fb: &Gray4Canvas, r: Rect, out: &mut heapless::Vec<u16, 4096>) {
    out.clear();
    let mut acc: u16 = 0;
    let mut nibbles = 0u8;
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let px = if x >= 0 && y >= 0 && x < fb.w && y < fb.h {
                fb.buf[y as usize * crate::canvas::FB_W + x as usize] as u16
            } else {
                0x0F
            };
            acc = (acc << 4) | (px & 0x0F);
            nibbles += 1;
            if nibbles == 4 {
                let _ = out.push(acc);
                acc = 0;
                nibbles = 0;
            }
        }
    }
    if nibbles != 0 {
        acc <<= 4 * (4 - nibbles as u16);
        let _ = out.push(acc);
    }
}

pub struct EinkRenderer<SPI, CS, RST, HRDY, DLY> {
    epd: It8951<SPI, CS, RST, HRDY, DLY>,
    target_addr: u32,
    pack: heapless::Vec<u16, 4096>,
}

impl<SPI, CS, RST, HRDY, DLY> EinkRenderer<SPI, CS, RST, HRDY, DLY>
where
    SPI: SpiBus,
    CS: OutputPin,
    RST: OutputPin,
    HRDY: InputPin,
    DLY: DelayNs,
{
    pub fn new(epd: It8951<SPI, CS, RST, HRDY, DLY>, target_addr: u32) -> Self {
        Self { epd, target_addr, pack: heapless::Vec::new() }
    }

    pub fn init(&mut self) {
        self.epd.init();
        self.epd.sleep();
    }
}

impl<SPI, CS, RST, HRDY, DLY> Renderer<Gray4Canvas> for EinkRenderer<SPI, CS, RST, HRDY, DLY>
where
    SPI: SpiBus,
    CS: OutputPin,
    RST: OutputPin,
    HRDY: InputPin,
    DLY: DelayNs,
{
    fn before_draw(&mut self) {
        self.epd.wake();
    }

    fn present(&mut self, canvas: &mut Gray4Canvas, region: Rect, hint: UpdateHint) {
        let x = region.x.max(0) as u16;
        let y = region.y.max(0) as u16;
        let w = region.w.max(0) as u16;
        let h = region.h.max(0) as u16;
        pack_region(canvas, region, &mut self.pack);
        self.epd.load_image_area(self.target_addr, x, y, w, h, &self.pack);
        self.epd.display_area(x, y, w, h, mode_for(hint));
    }

    fn after_draw(&mut self) {
        self.epd.sleep();
    }
}
