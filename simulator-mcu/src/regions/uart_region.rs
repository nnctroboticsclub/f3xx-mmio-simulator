use super::MmioHandler;
use crate::simulator::DynDevice;
use devconsole::ChannelID;
use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};

const UART_REGION_SIZE: usize = 0x2C;

pub struct UARTRegion {
    dev: DynDevice,
    start_addr: usize,
    peripheral_index: u8,
    channel_id: ChannelID,
    baud_rate_divisor: u32,
    cr1: u32,
    rx_queue: Arc<Mutex<VecDeque<u8>>>,
}
impl UARTRegion {
    fn new(dev: DynDevice, start_addr: usize, peripheral_index: u8) -> Self {
        let channel_id = dev.open_channel(format!("UART{}", peripheral_index));
        let rx_queue = Arc::new(Mutex::new(VecDeque::new()));
        if channel_id != 0 {
            let (tx, rx) = mpsc::channel();
            dev.listen(channel_id, None, Some(tx));
            let q = rx_queue.clone();
            std::thread::spawn(move || {
                while let Ok((_, data)) = rx.recv() {
                    let mut q = q.lock().unwrap();
                    for b in data {
                        q.push_back(b);
                    }
                }
            });
        }
        Self {
            dev,
            start_addr,
            peripheral_index,
            channel_id,
            baud_rate_divisor: 0,
            cr1: 0,
            rx_queue,
        }
    }
    pub fn new_boxed(
        dev: DynDevice,
        start_addr: usize,
        peripheral_index: u8,
    ) -> Box<dyn MmioHandler + Send + Sync> {
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

        if offset == 0x1C {
            let mut isr = 0x0000_00C0; // TXE=1, TC=1
            if !self.rx_queue.lock().unwrap().is_empty() {
                isr |= 1 << 5;
            } // RXNE
            return isr;
        }

        if offset == 0x24 {
            return self.rx_queue.lock().unwrap().pop_front().unwrap_or(0) as u32;
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

        if offset == 0x28 {
            if self.channel_id != 0 {
                self.dev.send_bin(self.channel_id, vec![value as u8]);
            }
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
