//! Minimal no_std IT8951 host-interface driver: the commands PaperUI needs
//! (system run/standby/sleep, register R/W, load image area, display area).
//! Implemented from the IT8951 SPI host-interface application note. UNVERIFIED on
//! hardware — see the crate's bring-up notes.

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;

const PREAMBLE_WRITE_CMD: u16 = 0x6000;
const PREAMBLE_WRITE_DATA: u16 = 0x0000;
const PREAMBLE_READ_DATA: u16 = 0x1000;

const CMD_SYS_RUN: u16 = 0x0001;
const CMD_STANDBY: u16 = 0x0002;
const CMD_SLEEP: u16 = 0x0003;
const CMD_REG_RD: u16 = 0x0010;
const CMD_REG_WR: u16 = 0x0011;
const CMD_LD_IMG_AREA: u16 = 0x0021;
const CMD_LD_IMG_END: u16 = 0x0022;
const CMD_DPY_AREA: u16 = 0x0034;

const REG_LISAR_L: u16 = 0x0200 + 0x08;
const REG_LISAR_H: u16 = 0x0200 + 0x0A;
const REG_LUTAFSR: u16 = 0x1000 + 0x224;

pub struct It8951<SPI, CS, RST, HRDY, DLY> {
    spi: SPI,
    cs: CS,
    rst: RST,
    hrdy: HRDY,
    delay: DLY,
}

impl<SPI, CS, RST, HRDY, DLY> It8951<SPI, CS, RST, HRDY, DLY>
where
    SPI: SpiBus,
    CS: OutputPin,
    RST: OutputPin,
    HRDY: InputPin,
    DLY: DelayNs,
{
    pub fn new(spi: SPI, cs: CS, rst: RST, hrdy: HRDY, delay: DLY) -> Self {
        Self { spi, cs, rst, hrdy, delay }
    }

    fn wait_ready(&mut self) {
        let mut spins = 0u32;
        while self.hrdy.is_low().unwrap_or(false) && spins < 2_000_000 {
            spins += 1;
        }
    }

    fn write_command(&mut self, cmd: u16) {
        self.wait_ready();
        let _ = self.cs.set_low();
        let mut pre = PREAMBLE_WRITE_CMD.to_be_bytes();
        let _ = self.spi.transfer_in_place(&mut pre);
        let mut b = cmd.to_be_bytes();
        self.wait_ready();
        let _ = self.spi.transfer_in_place(&mut b);
        let _ = self.cs.set_high();
    }

    fn write_data(&mut self, data: &[u16]) {
        self.wait_ready();
        let _ = self.cs.set_low();
        let mut pre = PREAMBLE_WRITE_DATA.to_be_bytes();
        let _ = self.spi.transfer_in_place(&mut pre);
        for &d in data {
            self.wait_ready();
            let mut b = d.to_be_bytes();
            let _ = self.spi.transfer_in_place(&mut b);
        }
        let _ = self.cs.set_high();
    }

    fn read_data(&mut self, out: &mut [u16]) {
        self.wait_ready();
        let _ = self.cs.set_low();
        let mut pre = PREAMBLE_READ_DATA.to_be_bytes();
        let _ = self.spi.transfer_in_place(&mut pre);
        let mut dummy = [0u8, 0u8];
        let _ = self.spi.transfer_in_place(&mut dummy);
        for w in out.iter_mut() {
            let mut b = [0u8, 0u8];
            let _ = self.spi.transfer_in_place(&mut b);
            *w = u16::from_be_bytes(b);
        }
        let _ = self.cs.set_high();
    }

    fn write_register(&mut self, reg: u16, val: u16) {
        self.write_command(CMD_REG_WR);
        self.write_data(&[reg, val]);
    }

    fn read_register(&mut self, reg: u16) -> u16 {
        self.write_command(CMD_REG_RD);
        self.write_data(&[reg]);
        let mut v = [0u16];
        self.read_data(&mut v);
        v[0]
    }

    pub fn init(&mut self) {
        let _ = self.rst.set_low();
        self.delay.delay_ms(50);
        let _ = self.rst.set_high();
        self.delay.delay_ms(50);
        self.wait_ready();
        self.write_command(CMD_SYS_RUN);
    }

    pub fn wake(&mut self) {
        self.write_command(CMD_SYS_RUN);
        self.wait_ready();
    }

    pub fn sleep(&mut self) {
        self.write_command(CMD_SLEEP);
    }

    /// Standby (lighter low-power state than sleep; faster wake). Provided for
    /// completeness of the IT8951 power command set.
    pub fn standby(&mut self) {
        self.write_command(CMD_STANDBY);
    }

    fn set_target_memory(&mut self, addr: u32) {
        self.write_register(REG_LISAR_H, (addr >> 16) as u16);
        self.write_register(REG_LISAR_L, (addr & 0xFFFF) as u16);
    }

    pub fn load_image_area(&mut self, addr: u32, x: u16, y: u16, w: u16, h: u16, packed: &[u16]) {
        self.set_target_memory(addr);
        self.write_command(CMD_LD_IMG_AREA);
        self.write_data(&[0x0000_u16 | (0x02 << 4), x, y, w, h]);
        self.write_data(packed);
        self.write_command(CMD_LD_IMG_END);
    }

    pub fn display_area(&mut self, x: u16, y: u16, w: u16, h: u16, mode: u16) {
        // Wait for the previous refresh to finish, but bound the spin so a wiring
        // fault or a wrong busy-register constant can't hang the firmware forever
        // (matches the bounded `wait_ready`).
        let mut spins = 0u32;
        while self.read_register(REG_LUTAFSR) != 0 && spins < 2_000_000 {
            spins += 1;
        }
        self.write_command(CMD_DPY_AREA);
        self.write_data(&[x, y, w, h, mode]);
    }
}
