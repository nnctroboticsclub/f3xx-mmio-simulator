use crate::simulator::DynDevice;

use super::MmioHandler;

const UART_REGION_SIZE: usize = 0x2C;

pub struct UARTRegion {
    start_addr: usize,
    peripheral_index: u8,
    baud_rate_divisor: u32,
    cr1: u32,
}
impl UARTRegion {
    fn new(_dev: DynDevice, start_addr: usize, peripheral_index: u8) -> Self {
        Self {
            start_addr,
            peripheral_index,
            baud_rate_divisor: 0,
            cr1: 0,
        }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize, peripheral_index: u8) -> Box<Self> {
        Box::new(Self::new(dev, start_addr, peripheral_index))
    }
    fn _is_transmitter_enabled(&self) -> bool {
        self.cr1 & 0x8 != 0
    }
    fn _is_receiver_enabled(&self) -> bool {
        self.cr1 & 0x4 != 0
    }
    fn _is_driver_enabled(&self) -> bool {
        self.cr1 & 0x1 != 0
    }
}

impl MmioHandler for UARTRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if offset == 0 {
            return self.cr1;
        }

        if offset == 0x1C {
            return self.baud_rate_divisor;
        }

        {
            panic!(
                "Read from undefined UART[{}] region at address {:08x}",
                self.peripheral_index, address
            );
        }
    }
    fn write(&mut self, address: usize, value: u64) {
        let value = value as u32;
        let offset = address - self.start_addr;

        if offset == 0x00 {
            let changed_bits = (self.cr1 ^ value) & 0xfffffff2;
            self.cr1 = value;

            if changed_bits != 0 {
                // We must process all changed bits to properly simulate the behavior of the hardware
                panic!(
                    "Write to UART[{}] CR1 register at address {:08x} with value {:08x} (changed bits: {:08x})",
                    self.peripheral_index, address, value, changed_bits
                );
            }

            return;
        }
        if offset == 0x0C {
            self.baud_rate_divisor = value;
            return;
        }

        {
            panic!(
                "Write to undefined UART[{}] region at address {:08x} with value {:08x}",
                self.peripheral_index, address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + UART_REGION_SIZE
    }
}
