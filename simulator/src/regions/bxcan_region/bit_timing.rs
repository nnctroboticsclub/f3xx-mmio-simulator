use crate::regions::bxcan_region::can_mode::CANMode;

pub struct BitTiming {
    mode: CANMode,
    sjw: u8,
    ts1: u8,
    ts2: u8,
    brp: u16,
}

impl BitTiming {
    pub fn new() -> Self {
        Self {
            mode: CANMode::Normal,
            sjw: 1,
            ts1: 4,
            ts2: 3,
            brp: 1,
        }
    }

    pub fn encode(&self) -> u32 {
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

    pub fn write(&mut self, value: u32) {
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
