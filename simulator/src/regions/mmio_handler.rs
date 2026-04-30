use std::fmt::Debug;

pub trait MmioHandler {
    fn read(&self, address: usize) -> u32;
    fn write(&mut self, address: usize, value: u32);
    fn contains(&self, address: usize) -> bool;
}

pub type DynMMIOHandler = Box<dyn MmioHandler + Send + Sync>;

impl Debug for DynMMIOHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynMMIOHandler")
    }
}
