use crate::simulator::DynDevice;

use super::MmioHandler;

const FLASH_REGION_SIZE: usize = 0x24;
pub struct FlashRegion {
    start_addr: usize,
    mem: [u8; FLASH_REGION_SIZE],
}
impl FlashRegion {
    fn new(_dev: DynDevice, start_addr: usize) -> Self {
        let mut mem = [0; FLASH_REGION_SIZE];
        mem[0] = 0x03;
        mem[0x20] = 0xFF;
        mem[0x21] = 0xFF;
        mem[0x22] = 0xFF;
        mem[0x23] = 0xFF;
        Self { start_addr, mem }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }
}

impl MmioHandler for FlashRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let defined_behavior = matches!(offset, 0x00..=0x03);
        if !defined_behavior {
            panic!(
                "Read from undefined flash region at address {:08x}",
                address
            );
        }
        let mut value = 0u32;
        for i in 0..4 {
            value |= (self.mem[offset + i] as u32) << (i * 8);
        }
        value
    }
    fn write(&mut self, address: usize, value: u64) {
        let offset = address - self.start_addr;
        if offset == 0x00 {
            // ACR
            self.mem[offset] = (value & 0xFF) as u8;
            self.mem[offset + 1] = ((value >> 8) & 0xFF) as u8;
            self.mem[offset + 2] = ((value >> 16) & 0xFF) as u8;
            self.mem[offset + 3] = ((value >> 24) & 0xFF) as u8;
        } else {
            panic!(
                "Write to undefined flash region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + FLASH_REGION_SIZE
    }
}
