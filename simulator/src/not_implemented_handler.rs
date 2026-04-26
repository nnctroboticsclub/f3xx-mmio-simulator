use crate::mmio_handler::MmioHandler;

struct NotImplementedHandler {
    start_addr: usize,
    end_addr: usize,
}
impl NotImplementedHandler {
    fn new(size: usize, start_addr: usize) -> Self {
        let end_addr = start_addr + size;
        Self {
            start_addr,
            end_addr,
        }
    }
    fn new_boxed(size: usize, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(size, start_addr))
    }
}
impl MmioHandler for NotImplementedHandler {
    fn read(&self, address: usize) -> u32 {
        panic!("Read from unimplemented MMIO address {:08x}", address);
    }
    fn write(&mut self, address: usize, value: u32) {
        panic!(
            "Write to unimplemented MMIO address {:08x} with value {:08x}",
            address, value
        );
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.end_addr
    }
}
