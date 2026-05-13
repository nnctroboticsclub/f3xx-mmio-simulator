mod bit_timing;
mod can_filter;
mod can_message;
mod can_mode;
mod can_state;
mod fifo_index;
mod filter_scale;
mod mailbox;
mod mailboxes;

use std::sync::{mpsc, Arc, Mutex};

use super::MmioHandler;
use crate::{
    regions::bxcan_region::{can_message::CANMessage, fifo_index::FIFOIndex, mailboxes::TXMailbox},
    simulator::DynDevice,
};
use bit_timing::BitTiming;
use can_filter::CANFilters;
use can_state::CANState;
use devconsole::ChannelID;
use mailbox::Mailbox;

const BXCAN_REGION_SIZE: usize = 0x2B0;

pub struct BXCanRegion {
    device: DynDevice,
    can_channel: ChannelID,

    start_addr: usize,
    state: CANState,
    bit_timing: BitTiming,
    filters: Arc<Mutex<CANFilters>>,
    tx_mailboxes: TXMailbox,
    rx_mailboxes: Arc<Mutex<[Mailbox; 2]>>,

    fifo0_pending_int_enable: bool,
    fifo1_pending_int_enable: bool,
}
impl BXCanRegion {
    async fn new(dev: DynDevice, start_addr: usize) -> Self {
        let (tx_bin, rx_bin) = mpsc::channel();
        let channel = dev.open_channel("can".to_string());
        dev.listen(channel, None, Some(tx_bin));

        let obj = Self {
            device: dev.clone(),
            can_channel: channel,
            start_addr,
            state: CANState::Sleep,
            bit_timing: BitTiming::new(),
            filters: Arc::new(Mutex::new(CANFilters::new())),
            tx_mailboxes: TXMailbox::new(),
            rx_mailboxes: Arc::new(Mutex::new([Mailbox::default(); 2])),
            fifo0_pending_int_enable: false,
            fifo1_pending_int_enable: false,
        };

        {
            let dev = obj.device.clone();
            let filters = obj.filters.clone();
            let rx_mailboxes = obj.rx_mailboxes.clone();
            std::thread::spawn(|| Self::dc_listener(dev, filters, rx_mailboxes, rx_bin));
        }
        obj
    }

    fn dc_listener(
        device: DynDevice,
        filters: Arc<Mutex<CANFilters>>,
        rx_mailboxes: Arc<Mutex<[Mailbox; 2]>>,
        rx: mpsc::Receiver<(ChannelID, Vec<u8>)>,
    ) {
        loop {
            let (_cid, msg) = rx.recv().unwrap();
            let msg = CANMessage::from(msg);
            let fifo = filters.lock().unwrap().route_message(msg.get_id(), true);
            if let Some(fifo) = fifo {
                let irqn = match fifo {
                    FIFOIndex::FIFO0 => 20,
                    FIFOIndex::FIFO1 => 21,
                };

                {
                    let mut mailboxes = rx_mailboxes.lock().unwrap();
                    let mailbox = if fifo == FIFOIndex::FIFO0 {
                        &mut mailboxes[0]
                    } else {
                        &mut mailboxes[1]
                    };
                    mailbox.store_message(msg);
                }

                device.fire_interrupt(16 + irqn);
            }
        }
    }

    pub async fn new_boxed(dev: DynDevice, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(dev, start_addr).await)
    }

    fn encode_mcr(&self) -> u32 {
        let mut mcr = 0x00010000; // DBF = 0

        if matches!(self.state, CANState::Sleep) {
            mcr |= 0x2;
        }

        if matches!(self.state, CANState::Initialization) {
            mcr |= 0x1;
        }

        mcr
    }

    fn encode_msr(&self) -> u32 {
        let mut msr = 0;

        if matches!(self.state, CANState::Sleep) {
            msr |= 0x2;
        }
        if matches!(self.state, CANState::Initialization) {
            msr |= 0x1;
        }

        msr
    }

    fn encode_ier(&self) -> u32 {
        let mut ier = 0;

        if self.fifo0_pending_int_enable {
            ier |= 0x02;
        }
        if self.fifo1_pending_int_enable {
            ier |= 0x10;
        }

        ier
    }
}

impl MmioHandler for BXCanRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if offset == 0x000 {
            return self.encode_mcr();
        }
        if offset == 0x004 {
            return self.encode_msr();
        }
        if offset == 0x00C {
            // Release FIFO 0 out: Always 0 (no FIFO implemented)
            // Overrun FIFO 0: Always 0 (no FIFO implemented)
            // Full FIFO 0: Always 0 (no FIFO implemented)
            // Pending FIFO0: Always 0
            return 0;
        }
        if let Some(value) = self.tx_mailboxes.read(offset) {
            return value;
        }
        if offset == 0x014 {
            return self.encode_ier();
        }
        if offset == 0x018 {
            return 0;
        }

        if offset == 0x01C {
            return self.bit_timing.encode();
        }

        if offset == 0x1B0 {
            return self.rx_mailboxes.lock().unwrap()[0].encode_tir();
        }
        if offset == 0x1B4 {
            return self.rx_mailboxes.lock().unwrap()[0].encode_tdtr();
        }
        if offset == 0x1B8 {
            return self.rx_mailboxes.lock().unwrap()[0].encode_tdlr();
        }
        if offset == 0x1BC {
            return self.rx_mailboxes.lock().unwrap()[0].encode_tdhr();
        }
        if offset == 0x1C0 {
            return self.rx_mailboxes.lock().unwrap()[1].encode_tir();
        }
        if offset == 0x1C4 {
            return self.rx_mailboxes.lock().unwrap()[1].encode_tdtr();
        }
        if offset == 0x1C8 {
            return self.rx_mailboxes.lock().unwrap()[1].encode_tdlr();
        }
        if offset == 0x1CC {
            return self.rx_mailboxes.lock().unwrap()[1].encode_tdhr();
        }

        if (0x200..=0x2B0).contains(&offset) {
            return self
                .filters
                .lock()
                .unwrap()
                .handle_read(offset - 0x200)
                .expect("bxCAN filter read handling should not fail");
        }

        {
            panic!(
                "Read from undefined bxCAN region at address {:08x}",
                address
            );
        }
    }
    fn write(&mut self, address: usize, value: u64) {
        let value = value as u32;
        let offset = address - self.start_addr;

        if let Some(msg) = self.tx_mailboxes.write(offset, value) {
            self.device.send_bin(self.can_channel, msg.into());
        }
        if (0x180..=0x1B0).contains(&offset) {
            return;
        }

        if offset == 0x000 {
            let mut changed_bits = (self.encode_mcr() ^ value) & 0x000080ff;

            if changed_bits & 0x00008000 != 0 && value & 0x00008000 != 0 {
                self.state = CANState::Sleep;
                changed_bits &= !0x00008000;
            }
            if changed_bits & 0x00000002 != 0 {
                self.state = if value & 0x00000002 != 0 {
                    CANState::Sleep
                } else {
                    CANState::Ready
                };
                changed_bits &= !0x00000002;
            }
            if changed_bits & 0x00000001 != 0 {
                self.state = if value & 0x00000001 != 0 {
                    CANState::Initialization
                } else {
                    CANState::Ready
                };
                changed_bits &= !0x00000001;
            }

            if changed_bits != 0 {
                panic!(
                    "Undefined write handling for CAN->MCR register changes. {:08x} -> {:08x} (changed bits: {:08x})",
                    self.encode_mcr(), value, changed_bits
                );
            }

            return;
        }

        if offset == 0x00C {
            return;
        }

        if offset == 0x014 {
            self.fifo0_pending_int_enable = value & 0x02 != 0;
            self.fifo1_pending_int_enable = value & 0x10 != 0;
            return;
        }

        if offset == 0x1C {
            self.bit_timing.write(value);
            return;
        }

        if offset == 0x1B0 {
            self.rx_mailboxes.lock().unwrap()[0].write_tir(value);
            return;
        }
        if offset == 0x1B4 {
            self.rx_mailboxes.lock().unwrap()[0].write_tdtr(value);
            return;
        }
        if offset == 0x1B8 {
            self.rx_mailboxes.lock().unwrap()[0].write_tdlr(value);
            return;
        }
        if offset == 0x1BC {
            self.rx_mailboxes.lock().unwrap()[0].write_tdhr(value);
            return;
        }
        if offset == 0x1C0 {
            self.rx_mailboxes.lock().unwrap()[1].write_tir(value);
            return;
        }
        if offset == 0x1C4 {
            self.rx_mailboxes.lock().unwrap()[1].write_tdtr(value);
            return;
        }
        if offset == 0x1C8 {
            self.rx_mailboxes.lock().unwrap()[1].write_tdlr(value);
            return;
        }
        if offset == 0x1CC {
            self.rx_mailboxes.lock().unwrap()[1].write_tdhr(value);
            return;
        }

        if (0x200..=0x2B0).contains(&offset) {
            self.filters
                .lock()
                .unwrap()
                .handle_write(offset - 0x200, value);
            return;
        }

        {
            panic!(
                "Write to undefined bxCAN region at +{:08x} with value {:08x}",
                offset, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + BXCAN_REGION_SIZE
    }
}
