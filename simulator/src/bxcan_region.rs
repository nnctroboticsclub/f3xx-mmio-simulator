use crate::mmio_handler::MmioHandler;

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

pub struct BXCanRegion {
    start_addr: usize,
    state: CANState,
    bit_timing: BitTiming,
    filter_state: CANFilterState,
    filters: [CANFilter; 13],

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
        if offset == 0x014 {
            return self.encode_ier();
        }
        if offset == 0x01C {
            return self.bit_timing.encode();
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
                println!("bxCAN state changed to {:?}", self.state);
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
