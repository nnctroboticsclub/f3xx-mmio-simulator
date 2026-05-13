use crate::simulator::DynDevice;
use crate::vector_table::{VectorTable, VectorTablePtr};

use super::MmioHandler;

pub struct BridgeRegion {
    start_addr: usize,
    dev: DynDevice,
}
impl BridgeRegion {
    fn new(dev: DynDevice, start_addr: usize) -> Self {
        Self { start_addr, dev }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr))
    }
}
impl MmioHandler for BridgeRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        if offset != 0 {
            panic!(
                "Read from undefined bridge region at address {:08x}",
                address
            );
        }
        let value = self.dev.get_vector_table().as_ptr() as usize as u32;
        value
    }
    fn write(&mut self, address: usize, value: u64) {
        let offset = address - self.start_addr;
        if offset != 0 {
            panic!(
                "Write to undefined bridge region at address {:08x} with value {:08x}",
                address, value
            );
        }
        self.dev
            .set_vector_table(VectorTablePtr::new(value as *const VectorTable));
    }
    fn contains(&self, address: usize) -> bool {
        self.start_addr <= address && address < self.start_addr + 8
    }
}
