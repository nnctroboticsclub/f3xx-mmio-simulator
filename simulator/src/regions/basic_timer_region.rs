use crate::simulator::DynDevice;

use super::MmioHandler;

const BASIC_TIMER_REGION_SIZE: usize = 0x30;

pub struct BasicTimerRegion {
    start_addr: usize,
    prescaler: u16,
    auto_reload_enabled: bool,
    auto_reload: u32,
    counter_enabled: bool,
    update_interrupt_enabled: bool,
}
impl BasicTimerRegion {
    fn new(_dev: DynDevice, start_addr: usize) -> Self {
        Self {
            start_addr,
            prescaler: 1,
            auto_reload_enabled: false,
            auto_reload: 0,
            counter_enabled: false,
            update_interrupt_enabled: false,
        }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }
    fn encode_cr1(&self) -> u32 {
        let mut value = 0;
        if self.counter_enabled {
            value |= 1 << 0; // CEN
        }
        if self.auto_reload_enabled {
            value |= 1 << 7; // ARPE
        }
        value
    }
}
impl MmioHandler for BasicTimerRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if offset == 0x00 {
            return self.encode_cr1(); // CR1
        }
        if offset == 0x0C {
            return if self.update_interrupt_enabled { 1 } else { 0 };
        }
        if offset == 0x28 {
            return self.prescaler as u32 - 1;
        }
        if offset == 0x2C {
            return self.auto_reload;
        }

        panic!(
            "Read from undefined BasicTimer region at address {:08x}",
            address
        );
    }
    fn write(&mut self, address: usize, value: u64) {
        let offset = address - self.start_addr;

        if offset == 0x00 {
            self.counter_enabled = (value & 1) != 0; // CEN
            self.auto_reload_enabled = (value & (1 << 7)) != 0;
            return;
        }

        if offset == 0x0C {
            self.update_interrupt_enabled = (value & 1) != 0;
            return;
        }

        if offset == 0x28 {
            self.prescaler = (value + 1) as u16;
            return;
        }

        if offset == 0x2C {
            self.auto_reload = value as u32;
            return;
        }

        panic!(
            "Write to undefined BasicTimer region at address {:08x}+{:04x} with value {:08x}",
            self.start_addr, offset, value
        );
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + BASIC_TIMER_REGION_SIZE
    }
}
