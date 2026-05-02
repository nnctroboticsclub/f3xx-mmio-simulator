use std::sync::Arc;

use crate::simulator::DynDevice;

use super::MmioHandler;

const RCC_REGION_SIZE: usize = 0x24;
const RCC_HSI_CLOCK: u32 = 8_000_000;
const RCC_HSE_CLOCK: u32 = 8_000_000;
enum PLLSource {
    HSIDiv2,
    HSE(u32), // prescaler
}
enum SysClockSource {
    HSI,
    HSE,
    PLL,
}
pub struct RCCRegion {
    start_addr: usize,
    mem: [u32; RCC_REGION_SIZE / 4],
    pll_source: PLLSource,
    pll_multiplier: u32,
    ahb_prescaler: u32,
    apb1_prescaler: u32,
    apb2_prescaler: u32,
    sysclk_source: SysClockSource,
}
impl RCCRegion {
    fn new(dev: DynDevice, start_addr: usize) -> Self {
        let mut mem = [0u32; RCC_REGION_SIZE / 4];
        mem[0] = 0x03300083; // PLL, HSE, HSI ON
        mem[5] = 0x00000014;
        mem[8] = 0x00000018;
        Self {
            start_addr,
            mem,
            pll_source: PLLSource::HSIDiv2,
            pll_multiplier: 2,
            ahb_prescaler: 1,
            apb1_prescaler: 1,
            apb2_prescaler: 1,
            sysclk_source: SysClockSource::HSI,
        }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }

    fn set_pll_multiplier_value(&mut self, value: u32) {
        if value == 0x0f {
            self.pll_multiplier = 16;
        } else {
            self.pll_multiplier = value + 2;
        }
    }

    fn get_pll_multiplier_value(&self) -> u32 {
        if self.pll_multiplier == 16 {
            0x0f
        } else {
            self.pll_multiplier - 2
        }
    }

    fn get_pll_hse_prediv_value(&self) -> Option<u32> {
        match self.pll_source {
            PLLSource::HSIDiv2 => None,
            PLLSource::HSE(n) => Some(n - 1),
        }
    }

    fn set_pll_hse_prediv_value(&mut self, value: u32) {
        match self.pll_source {
            PLLSource::HSIDiv2 => {
                panic!("Cannot set HSE predivider when PLL source is HSI/2");
            }
            PLLSource::HSE(_) => {
                self.pll_source = PLLSource::HSE(value + 1);
            }
        }
    }

    fn get_pll_source(&self) -> u32 {
        match self.pll_source {
            PLLSource::HSIDiv2 => 0,
            PLLSource::HSE(_) => 1,
        }
    }

    fn set_pll_source(&mut self, value: u32) {
        match value {
            0 => self.pll_source = PLLSource::HSIDiv2,
            1 => self.pll_source = PLLSource::HSE(1), // default to HSE with no predivision
            _ => panic!("Invalid PLL source value: {value}"),
        }
    }

    fn get_apb1_prescaler_value(&self) -> u32 {
        match self.apb1_prescaler {
            1 => 3,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid APB1 prescaler value: {}", self.apb1_prescaler),
        }
    }
    fn set_apb1_prescaler_value(&mut self, value: u32) {
        self.apb1_prescaler = match value {
            0..=3 => 1,
            4 => 2,
            5 => 4,
            6 => 8,
            7 => 16,
            _ => panic!("Invalid APB1 prescaler setting: {value}"),
        }
    }

    fn get_apb2_prescaler_value(&self) -> u32 {
        match self.apb2_prescaler {
            1 => 3,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid APB2 prescaler value: {}", self.apb2_prescaler),
        }
    }
    fn set_apb2_prescaler_value(&mut self, value: u32) {
        self.apb2_prescaler = match value {
            0..=3 => 1,
            4 => 2,
            5 => 4,
            6 => 8,
            7 => 16,
            _ => panic!("Invalid APB2 prescaler setting: {value}"),
        }
    }
    fn get_ahb_prescaler_value(&self) -> u32 {
        match self.ahb_prescaler {
            1 => 7,
            2 => 8,
            4 => 9,
            8 => 10,
            16 => 11,
            64 => 12,
            128 => 13,
            256 => 14,
            512 => 15,
            _ => panic!("Invalid AHB prescaler value: {}", self.ahb_prescaler),
        }
    }
    fn set_ahb_prescaler_value(&mut self, value: u32) {
        self.ahb_prescaler = match value {
            0..=7 => 1,
            8 => 2,
            9 => 4,
            10 => 8,
            11 => 16,
            12 => 64,
            13 => 128,
            14 => 256,
            15 => 512,
            _ => panic!("Invalid AHB prescaler setting: {value}"),
        }
    }
    fn get_sysclk_source(&self) -> u32 {
        match self.sysclk_source {
            SysClockSource::HSI => 0,
            SysClockSource::HSE => 1,
            SysClockSource::PLL => 2,
        }
    }
    fn set_sysclk_source(&mut self, value: u32) {
        self.sysclk_source = match value {
            0 => SysClockSource::HSI,
            1 => SysClockSource::HSE,
            2 => SysClockSource::PLL,
            _ => panic!("Invalid system clock source value: {value}"),
        }
    }
}
impl MmioHandler for RCCRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let value = match offset {
            0x00 => self.mem[0], // CR
            0x04 => {
                let mut reg = 0;
                reg |= self.get_pll_multiplier_value() << 18;
                reg |= (self.get_pll_hse_prediv_value().unwrap_or(0) & 1) << 17;
                reg |= self.get_pll_source() << 15;
                reg |= self.get_apb1_prescaler_value() << 11;
                reg |= self.get_apb2_prescaler_value() << 8;
                reg |= self.get_ahb_prescaler_value() << 4;
                reg |= self.get_sysclk_source() << 2;
                reg |= self.get_sysclk_source();
                reg
            }
            0x14 | 0x18 | 0x1C => self.mem[offset / 4],
            0x18 => self.mem[6], // APB2 ENR
            0x1C => self.mem[7], // APB1 ENR
            _ => {
                panic!("Read from undefined RCC region at address {:08x}", address);
            }
        };
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;

        let original_value = self.mem[offset / 4];
        let new_value = value;

        let mut changed_bits = original_value ^ new_value;

        if offset == 0x00 {
            changed_bits &= 0x02040003; // CR ignores PLLON/HSRON/CSSON

            if changed_bits != 0 {
                panic!(
                    "Unhandled RCC->CR write: {:08x} ==> {:08x}",
                    original_value, new_value
                );
            }
        } else if offset == 0x04 {
            changed_bits &= 0x00c3fff3;
            self.mem[offset / 4] = new_value;
            if changed_bits & 0x003c0000 != 0 {
                self.set_pll_multiplier_value((new_value >> 18) & 0xf);
                changed_bits &= !0x003c0000;
            }
            if changed_bits & 0x00000700 != 0 {
                let ppre1 = (new_value >> 8) & 0x7;
                self.set_apb1_prescaler_value(ppre1);
                changed_bits &= !0x00000700;
            }
            if changed_bits & 0x00003800 != 0 {
                let ppre2 = (new_value >> 11) & 0x7;
                self.set_apb2_prescaler_value(ppre2);
                changed_bits &= !0x00003800;
            }
            if changed_bits & 0x000000f0 != 0 {
                let hpre = (new_value >> 4) & 0xf;
                self.set_ahb_prescaler_value(hpre);
                changed_bits &= !0x000000f0;
            }
            if changed_bits & 0x00000003 != 0 {
                self.set_sysclk_source(new_value & 0x3);
                changed_bits &= !0x00000003;
            }
            if changed_bits != 0 {
                panic!(
                    "Unhandled RCC->CFGR write: {:08x} ==> {:08x}",
                    original_value, new_value
                );
            }
        } else if offset == 0x14 || offset == 0x18 || offset == 0x1C {
            // AHB ENR, APB2 ENR, APB1 ENR
            // We can ignore this since we don't simulate individual peripherals
        } else {
            panic!(
                "Write to undefined RCC region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + RCC_REGION_SIZE
    }
}
