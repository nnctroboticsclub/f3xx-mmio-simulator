use std::sync::Arc;

use crate::simulator::Device;

use super::MmioHandler;

const NVIC_REGION_SIZE: usize = 0x400;

#[derive(Copy, Clone)]
struct InterruptionConfig {
    priority: u8,
    enabled: bool,
    pending: bool,
}

pub struct NVICRegion {
    start_addr: usize,
    interrupts: [InterruptionConfig; 240],
    active_interrupt: Option<u8>,
}
impl NVICRegion {
    fn new(dev: Arc<Device>, start_addr: usize) -> Self {
        Self {
            start_addr,
            interrupts: [InterruptionConfig {
                priority: 0,
                enabled: false,
                pending: false,
            }; 240],
            active_interrupt: None,
        }
    }
    pub fn new_boxed(dev: Arc<Device>, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }
}
impl MmioHandler for NVICRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if 0 <= offset && offset <= 0x100 {
            let reg_index = (offset & 0x7F) / 4;
            let mut value = 0;
            for i in 0..32 {
                let int_index = reg_index * 32 + i;
                if int_index < 240 {
                    let config = self.interrupts[int_index];
                    let bit = (config.enabled as u32) << 1 | (config.pending as u32);
                    value |= bit << i;
                }
            }
            return value;
        }

        panic!("Read from undefined NVIC region at address {:08x}", address);
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;

        if offset <= 0x80 {
            let reg_index = (offset & 0x7F) / 4;
            for i in 0..32 {
                let int_index = reg_index * 32 + i;
                let bit = (value >> i) & 1;
                if bit == 0 {
                    continue;
                }
                self.interrupts[int_index].enabled = true;
            }
            return;
        }
        if 0x80 <= offset && offset <= 0x100 {
            let reg_index = (offset & 0x7F) / 4;
            for i in 0..32 {
                let int_index = reg_index * 32 + i;
                let bit = (value >> i) & 1;
                if bit == 0 {
                    continue;
                }
                self.interrupts[int_index].enabled = false;
            }
            return;
        }
        if 0x300 <= offset && offset <= 0x400 {
            let reg_index = (offset & 0xFF) / 4;
            for i in 0..4 {
                let int_index = reg_index * 4 + i;
                let val = (value >> (i * 8)) & 0xFF;
                if val == 0 {
                    continue;
                }
                self.interrupts[int_index].priority = val as u8;
            }
            return;
        }

        panic!(
            "Write to undefined NVIC region at address {:08x}+{:04x} with value {:08x}",
            self.start_addr, offset, value
        );
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + NVIC_REGION_SIZE
    }
}
