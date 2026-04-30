use super::MmioHandler;
use crate::vector_table::VectorTable;

pub struct BridgeRegion {
    start_addr: usize,
    vtor: usize,
}
impl BridgeRegion {
    fn new(start_addr: usize) -> Self {
        Self {
            start_addr,
            vtor: 0,
        }
    }
    pub fn new_boxed(start_addr: usize) -> Box<Self> {
        Box::new(Self::new(start_addr))
    }

    pub fn get_registered_vector_table(&self) -> *mut VectorTable {
        self.vtor as *mut VectorTable
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
        let value = self.vtor as usize as u32;
        println!("R {address:08x} --> {value:08x}");
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;
        if offset != 0 {
            panic!(
                "Write to undefined bridge region at address {:08x} with value {:08x}",
                address, value
            );
        }
        self.vtor = value as usize;
        println!("W {address:08x} <-- {value:08x}");
    }
    fn contains(&self, address: usize) -> bool {
        self.start_addr <= address && address < self.start_addr + 8
    }
}
