use std::sync::Arc;

use crate::simulator::Device;

use super::MmioHandler;

struct MemHandler {
    mem: Vec<u8>,
    start_addr: usize,
    end_addr: usize,
}
impl MemHandler {
    fn new(dev: Arc<Device>, size: usize, start_addr: usize) -> Self {
        let end_addr = start_addr + size;
        Self {
            mem: vec![0; size],
            start_addr,
            end_addr,
        }
    }
    fn new_boxed(dev: Arc<Device>, size: usize, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, size, start_addr))
    }
}
impl MmioHandler for MemHandler {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let mut value = 0u32;
        for i in 0..4 {
            value |= (self.mem[offset + i] as u32) << (i * 8);
        }
        println!("R {address:08x} --> {value:08x}");
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;
        println!("W {address:08x} <-- {value:08x}");
        for i in 0..4 {
            self.mem[offset + i] = ((value >> (i * 8)) & 0xFF) as u8;
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.end_addr
    }
}
