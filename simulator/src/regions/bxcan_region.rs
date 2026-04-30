use super::MmioHandler;

const BXCAN_REGION_SIZE: usize = 0x2B0;

#[derive(Debug)]
enum CANState {
    Sleep,
    Initialization,
    Ready,
}

enum CANMode {
    Normal,
    Loopback,
    Silent,
    SilentLoopback,
}

struct BitTiming {
    mode: CANMode,
    sjw: u8,
    ts1: u8,
    ts2: u8,
    brp: u16,
}

impl BitTiming {
    fn new() -> Self {
        Self {
            mode: CANMode::Normal,
            sjw: 1,
            ts1: 4,
            ts2: 3,
            brp: 1,
        }
    }

    fn encode(&self) -> u32 {
        let mut btr = 0;

        btr |= match self.mode {
            CANMode::Normal => 0,
            CANMode::Loopback => 0x40000000,
            CANMode::Silent => 0x80000000,
            CANMode::SilentLoopback => 0xC0000000,
        };
        btr |= ((self.sjw - 1) as u32) << 24;
        btr |= ((self.ts1 - 1) as u32) << 16;
        btr |= ((self.ts2 - 1) as u32) << 20;
        btr |= (self.brp - 1) as u32;

        return btr;
    }

    fn write(&mut self, value: u32) {
        let mode_bits = value & 0xC0000000;
        self.mode = match mode_bits {
            0 => CANMode::Normal,
            0x40000000 => CANMode::Loopback,
            0x80000000 => CANMode::Silent,
            0xC0000000 => CANMode::SilentLoopback,
            _ => unreachable!(),
        };

        self.sjw = ((value >> 24) & 0x3F) as u8 + 1;
        self.ts1 = ((value >> 16) & 0x0F) as u8 + 1;
        self.ts2 = ((value >> 20) & 0x07) as u8 + 1;
        self.brp = (value & 0x3FF) as u16 + 1;
    }
}

enum CANFilterState {
    Init,
    Active,
}

#[derive(Clone, Copy)]
enum FIFOIndex {
    FIFO0,
    FIFO1,
}

#[derive(Clone, Copy)]
enum FilterScale {
    Scale16Bit,
    Scale32Bit,
}

#[derive(Clone, Copy)]
enum CANFilterType {
    Mask,
    List,
}

#[derive(Clone, Copy)]
struct CANFilter {
    activated: bool,
    fifo_assignment: FIFOIndex,
    scale: FilterScale,
    bank0: u32,
    bank1: u32,
    filter_type: CANFilterType,
}

impl CANFilter {
    fn new() -> Self {
        Self {
            activated: false,
            fifo_assignment: FIFOIndex::FIFO0,
            scale: FilterScale::Scale16Bit,
            bank0: 0,
            bank1: 0,
            filter_type: CANFilterType::Mask,
        }
    }
}

#[derive(Clone, Copy)]
enum MailboxState {
    Init,
    Pending(usize),
    Working,
    Done,
    DoneError,
}

#[derive(Copy, Clone)]
struct Mailbox {
    id: u32,
    ide: bool,
    rtr: bool,
    dlc: u8,
    data: [u8; 8],
    state: MailboxState,
}

impl Mailbox {
    fn new() -> Self {
        Self {
            id: 0,
            ide: false,
            rtr: false,
            dlc: 0,
            data: [0; 8],
            state: MailboxState::Init,
        }
    }

    fn encode_tir(&self) -> u32 {
        0 | if self.ide {
            (self.id & 0x1FFFFFFF) << 3 | 0x4
        } else {
            (self.id & 0x7FF) << 21 | 0x4
        } | if self.rtr { 0x2 } else { 0 }
    }

    fn encode_tdtr(&self) -> u32 {
        (self.dlc as u32) & 0xF
    }

    fn encode_tdlr(&self) -> u32 {
        (self.data[0] as u32)
            | ((self.data[1] as u32) << 8)
            | ((self.data[2] as u32) << 16)
            | ((self.data[3] as u32) << 24)
    }

    fn encode_tdhr(&self) -> u32 {
        (self.data[4] as u32)
            | ((self.data[5] as u32) << 8)
            | ((self.data[6] as u32) << 16)
            | ((self.data[7] as u32) << 24)
    }

    fn encode_tsr(&self) -> u8 {
        match self.state {
            MailboxState::Init => 0x00,
            MailboxState::Pending(_) => 0x00,
            MailboxState::Working => 0x00,
            MailboxState::Done => 0x03,
            MailboxState::DoneError => 0x09,
        }
    }

    fn abort(&mut self) {
        if let MailboxState::Pending(_) | MailboxState::Working = self.state {
            self.state = MailboxState::Init;
        }
    }

    fn is_empty(&self) -> bool {
        match self.state {
            MailboxState::Init => true,
            MailboxState::Done => true,
            MailboxState::DoneError => true,
            _ => false,
        }
    }
    fn write_tir(&mut self, value: u32) {
        self.ide = (value & 0x4) != 0;
        self.rtr = (value & 0x2) != 0;
        if self.ide {
            self.id = (value >> 3) & 0x1FFFFFFF;
        } else {
            self.id = (value >> 21) & 0x7FF;
        }
        if value & 1 != 0 {
            self.state = MailboxState::Pending(0);
        }
    }
    fn write_tdtr(&mut self, value: u32) {
        self.dlc = (value & 0xF) as u8;
    }
    fn write_tdlr(&mut self, value: u32) {
        self.data[0] = (value & 0xFF) as u8;
        self.data[1] = ((value >> 8) & 0xFF) as u8;
        self.data[2] = ((value >> 16) & 0xFF) as u8;
        self.data[3] = ((value >> 24) & 0xFF) as u8;
    }
    fn write_tdhr(&mut self, value: u32) {
        self.data[4] = (value & 0xFF) as u8;
        self.data[5] = ((value >> 8) & 0xFF) as u8;
        self.data[6] = ((value >> 16) & 0xFF) as u8;
        self.data[7] = ((value >> 24) & 0xFF) as u8;
    }
}

pub struct BXCanRegion {
    start_addr: usize,
    state: CANState,
    bit_timing: BitTiming,
    filter_state: CANFilterState,
    filters: [CANFilter; 13],
    tx_mailboxes: [Mailbox; 3],
    rx_mailboxes: [Mailbox; 2],

    fifo0_pending_int_enable: bool,
    fifo1_pending_int_enable: bool,
}
impl BXCanRegion {
    fn new(start_addr: usize) -> Self {
        Self {
            start_addr,
            state: CANState::Sleep,
            bit_timing: BitTiming::new(),
            filter_state: CANFilterState::Init,
            filters: [CANFilter::new(); 13],
            tx_mailboxes: [Mailbox::new(); 3],
            rx_mailboxes: [Mailbox::new(); 2],
            fifo0_pending_int_enable: false,
            fifo1_pending_int_enable: false,
        }
    }
    pub fn new_boxed(start_addr: usize) -> Box<Self> {
        Box::new(Self::new(start_addr))
    }

    fn encode_mcr(&self) -> u32 {
        let mut mcr = 0x00010000; // DBF = 0

        if matches!(self.state, CANState::Sleep) {
            mcr |= 0x2;
        }

        if matches!(self.state, CANState::Initialization) {
            mcr |= 0x1;
        }

        return mcr;
    }

    fn encode_msr(&self) -> u32 {
        let mut msr = 0;

        if matches!(self.state, CANState::Sleep) {
            msr |= 0x2;
        }
        if matches!(self.state, CANState::Initialization) {
            msr |= 0x1;
        }

        return msr;
    }

    fn encode_ier(&self) -> u32 {
        let mut ier = 0;

        if self.fifo0_pending_int_enable {
            ier |= 0x02;
        }
        if self.fifo1_pending_int_enable {
            ier |= 0x10;
        }

        return ier;
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
        if offset == 0x008 {
            let mut tsr = 0;
            let lowest_priority_mailbox = self
                .tx_mailboxes
                .iter()
                .filter(|mb| matches!(mb.state, MailboxState::Pending(_)))
                .enumerate()
                .max_by_key(|&(_, mb)| match mb.state {
                    MailboxState::Pending(priority) => priority,
                    _ => usize::MAX,
                })
                .map(|(i, _)| i);
            let next_free_mailbox = self
                .tx_mailboxes
                .iter()
                .position(|mb| mb.is_empty())
                .map(|x| x as u32);
            if let Some(lowest_priority_mailbox) = lowest_priority_mailbox {
                tsr |= 0x20 << lowest_priority_mailbox;
            }
            if let Some(next_free_mailbox) = next_free_mailbox {
                tsr |= next_free_mailbox << 24;
            }
            for i in 0..3 {
                tsr |= (self.tx_mailboxes[i].encode_tsr() as u32) << (i * 8);
                tsr |= if self.tx_mailboxes[i].is_empty() {
                    1 << (26 + i)
                } else {
                    0
                };
            }
            return tsr;
        }
        if offset == 0x014 {
            return self.encode_ier();
        }
        if offset == 0x18 {
            return 0; // ESR, 0 for no error
        }
        if offset == 0x01C {
            return self.bit_timing.encode();
        }
        if offset == 0x180 {
            return self.tx_mailboxes[0].encode_tir();
        }
        if offset == 0x184 {
            return self.tx_mailboxes[0].encode_tdtr();
        }
        if offset == 0x188 {
            return self.tx_mailboxes[0].encode_tdlr();
        }
        if offset == 0x18C {
            return self.tx_mailboxes[0].encode_tdhr();
        }
        if offset == 0x190 {
            return self.tx_mailboxes[1].encode_tir();
        }
        if offset == 0x194 {
            return self.tx_mailboxes[1].encode_tdtr();
        }
        if offset == 0x198 {
            return self.tx_mailboxes[1].encode_tdlr();
        }
        if offset == 0x19C {
            return self.tx_mailboxes[1].encode_tdhr();
        }
        if offset == 0x1A0 {
            return self.tx_mailboxes[2].encode_tir();
        }
        if offset == 0x1A4 {
            return self.tx_mailboxes[2].encode_tdtr();
        }
        if offset == 0x1A8 {
            return self.tx_mailboxes[2].encode_tdlr();
        }
        if offset == 0x1AC {
            return self.tx_mailboxes[2].encode_tdhr();
        }
        if offset == 0x1B0 {
            return self.rx_mailboxes[0].encode_tir();
        }
        if offset == 0x1B4 {
            return self.rx_mailboxes[0].encode_tdtr();
        }
        if offset == 0x1B8 {
            return self.rx_mailboxes[0].encode_tdlr();
        }
        if offset == 0x1BC {
            return self.rx_mailboxes[0].encode_tdhr();
        }
        if offset == 0x1C0 {
            return self.rx_mailboxes[1].encode_tir();
        }
        if offset == 0x1C4 {
            return self.rx_mailboxes[1].encode_tdtr();
        }
        if offset == 0x1C8 {
            return self.rx_mailboxes[1].encode_tdlr();
        }
        if offset == 0x1CC {
            return self.rx_mailboxes[1].encode_tdhr();
        }
        if offset == 0x200 {
            return 0x2A1C0E00
                | match self.filter_state {
                    CANFilterState::Init => 0,
                    CANFilterState::Active => 1,
                };
        }
        if offset == 0x204 {
            let mut value = 0;
            for i in 0..13 {
                if let CANFilterType::List = self.filters[i].filter_type {
                    value |= 1 << i;
                }
            }
            return value;
        }
        if offset == 0x20C {
            let mut value = 0;
            for i in 0..13 {
                if let FilterScale::Scale32Bit = self.filters[i].scale {
                    value |= 1 << i;
                }
            }
            return value;
        }
        if offset == 0x214 {
            let mut value = 0;
            for i in 0..13 {
                if let FIFOIndex::FIFO1 = self.filters[i].fifo_assignment {
                    value |= 1 << i;
                }
            }
            return value;
        }
        if offset == 0x21C {
            let mut value = 0;
            for i in 0..13 {
                if self.filters[i].activated {
                    value |= 1 << i;
                }
            }
            return value;
        }

        if 0x240 <= offset && offset <= 0x2AC {
            let sub_offset = offset - 0x240;
            let filter_id = sub_offset / 2;
            let bank_part = sub_offset % 2;
            if filter_id >= 13 {
                panic!(
                    "Read from undefined bxCAN filter register at address {:08x}",
                    address
                );
            }

            let filter = self.filters[filter_id];
            let value = match bank_part {
                0 => filter.bank0,
                1 => filter.bank1,
                _ => unreachable!(),
            };
            return value;
        }

        {
            panic!(
                "Read from undefined bxCAN region at address {:08x}",
                address
            );
        }
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;

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

        if offset == 0x014 {
            self.fifo0_pending_int_enable = value & 0x02 != 0;
            self.fifo1_pending_int_enable = value & 0x10 != 0;
            return;
        }

        if offset == 0x1C {
            self.bit_timing.write(value);
            return;
        }

        if offset == 0x180 {
            self.tx_mailboxes[0].write_tir(value);
            return;
        }
        if offset == 0x184 {
            self.tx_mailboxes[0].write_tdtr(value);
            return;
        }
        if offset == 0x188 {
            self.tx_mailboxes[0].write_tdlr(value);
            return;
        }
        if offset == 0x18C {
            self.tx_mailboxes[0].write_tdhr(value);
            return;
        }
        if offset == 0x190 {
            self.tx_mailboxes[1].write_tir(value);
            return;
        }
        if offset == 0x194 {
            self.tx_mailboxes[1].write_tdtr(value);
            return;
        }
        if offset == 0x198 {
            self.tx_mailboxes[1].write_tdlr(value);
            return;
        }
        if offset == 0x19C {
            self.tx_mailboxes[1].write_tdhr(value);
            return;
        }
        if offset == 0x1A0 {
            self.tx_mailboxes[2].write_tir(value);
            return;
        }
        if offset == 0x1A4 {
            self.tx_mailboxes[2].write_tdtr(value);
            return;
        }
        if offset == 0x1A8 {
            self.tx_mailboxes[2].write_tdlr(value);
            return;
        }
        if offset == 0x1AC {
            self.tx_mailboxes[2].write_tdhr(value);
            return;
        }
        if offset == 0x1B0 {
            self.rx_mailboxes[0].write_tir(value);
            return;
        }
        if offset == 0x1B4 {
            self.rx_mailboxes[0].write_tdtr(value);
            return;
        }
        if offset == 0x1B8 {
            self.rx_mailboxes[0].write_tdlr(value);
            return;
        }
        if offset == 0x1BC {
            self.rx_mailboxes[0].write_tdhr(value);
            return;
        }
        if offset == 0x1C0 {
            self.rx_mailboxes[1].write_tir(value);
            return;
        }
        if offset == 0x1C4 {
            self.rx_mailboxes[1].write_tdtr(value);
            return;
        }
        if offset == 0x1C8 {
            self.rx_mailboxes[1].write_tdlr(value);
            return;
        }
        if offset == 0x1CC {
            self.rx_mailboxes[1].write_tdhr(value);
            return;
        }

        if offset == 0x200 {
            if value & 0xFFFFFFFE != 0x2A1C0E00 {
                panic!(
                    "Write to bxCAN filter register with unsupported value {:08x}",
                    value
                );
            }
            if value & 0x1 != 0 {
                self.filter_state = CANFilterState::Active;
            } else {
                self.filter_state = CANFilterState::Init;
            }
            return;
        }

        if offset == 0x204 {
            for i in 0..13 {
                let filter_id = i;
                self.filters[i].filter_type = if value & (1 << filter_id) != 0 {
                    CANFilterType::List
                } else {
                    CANFilterType::Mask
                };
            }
            return;
        }

        if offset == 0x20C {
            if value & 0xFFFFFC00 != 0 {
                panic!(
                    "Write to bxCAN filter scale register with unsupported value {:08x}",
                    value
                );
            }
            for i in 0..13 {
                let filter_id = i;
                self.filters[i].scale = if value & (1 << filter_id) != 0 {
                    FilterScale::Scale32Bit
                } else {
                    FilterScale::Scale16Bit
                };
            }
            return;
        }

        if offset == 0x214 {
            if value & 0xFFFFFE00 != 0 {
                panic!(
                    "Write to bxCAN filter FIFO assignment register with unsupported value {:08x}",
                    value
                );
            }
            for i in 0..13 {
                let filter_id = i;
                self.filters[i].fifo_assignment = if value & (1 << filter_id) != 0 {
                    FIFOIndex::FIFO1
                } else {
                    FIFOIndex::FIFO0
                };
            }
            return;
        }

        if offset == 0x21C {
            if value & 0xFFFFC000 != 0 {
                panic!(
                    "Write to bxCAN filter activation register with unsupported value {:08x}",
                    value
                );
            }
            for i in 0..13 {
                let filter_id = i;
                let activated = (value & (1 << filter_id)) != 0;
                self.filters[i].activated = activated;
            }
            return;
        }

        if 0x240 <= offset && offset <= 0x2AC {
            let sub_offset = offset - 0x240;
            let filter_id = sub_offset / 2;
            let bank_part = sub_offset % 2;
            if filter_id >= 13 {
                panic!(
                    "Write to undefined bxCAN filter register at address {:08x} with value {:08x}",
                    address, value
                );
            }

            let filter = &mut self.filters[filter_id];
            match bank_part {
                0 => filter.bank0 = value,
                1 => filter.bank1 = value,
                _ => unreachable!(),
            };
            return;
        }

        {
            panic!(
                "Write to undefined bxCAN region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + BXCAN_REGION_SIZE
    }
}
