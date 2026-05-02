use std::sync::Arc;

use crate::simulator::Device;

use super::MmioHandler;

const SCB_REGION_SIZE: usize = 0x40;

pub struct SCBRegion {
    start_addr: usize,
}
impl SCBRegion {
    fn new(dev: Arc<Device>, start_addr: usize) -> Self {
        Self { start_addr }
    }
    pub fn new_boxed(dev: Arc<Device>, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }
}
impl MmioHandler for SCBRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if offset == 0x0c {
            return 0xFA050000; // AIRCR
        }

        panic!("Read from undefined SCB region at address {:08x}", address);
    }
    fn write(&mut self, address: usize, value: u32) {
        panic!(
            "Write to undefined SCB region at address {:08x} with value {:08x}",
            address, value
        );
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + SCB_REGION_SIZE
    }
}
