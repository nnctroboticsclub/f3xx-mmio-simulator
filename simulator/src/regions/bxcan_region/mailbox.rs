use crate::regions::bxcan_region::can_message::CANMessage;

#[derive(Copy, Clone)]
pub struct Mailbox {
    id: u32,
    ide: bool,
    rtr: bool,
    dlc: u8,
    data: [u8; 8],

    after_sent: bool,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            id: 0,
            ide: false,
            rtr: false,
            dlc: 0,
            data: [0; 8],
            after_sent: false,
        }
    }

    pub fn encode_tir(&self) -> u32 {
        0 | if self.ide {
            (self.id & 0x1FFFFFFF) << 3 | 0x4
        } else {
            (self.id & 0x7FF) << 21 | 0x4
        } | if self.rtr { 0x2 } else { 0 }
    }

    pub fn encode_tdtr(&self) -> u32 {
        (self.dlc as u32) & 0xF
    }

    pub fn encode_tdlr(&self) -> u32 {
        (self.data[0] as u32)
            | ((self.data[1] as u32) << 8)
            | ((self.data[2] as u32) << 16)
            | ((self.data[3] as u32) << 24)
    }

    pub fn encode_tdhr(&self) -> u32 {
        (self.data[4] as u32)
            | ((self.data[5] as u32) << 8)
            | ((self.data[6] as u32) << 16)
            | ((self.data[7] as u32) << 24)
    }

    pub fn encode_tsr(&self) -> u8 {
        match self.after_sent {
            false => 0x00,
            true => 0x03,
        }
    }

    pub fn abort(&mut self) {
        self.after_sent = true;
    }

    pub fn is_empty(&self) -> bool {
        true
    }
    pub fn write_tir(&mut self, value: u32) {
        self.ide = (value & 0x4) != 0;
        self.rtr = (value & 0x2) != 0;
        if self.ide {
            self.id = (value >> 3) & 0x1FFFFFFF;
        } else {
            self.id = (value >> 21) & 0x7FF;
        }
        if value & 1 != 0 {
            self.after_sent = true;
        }
    }
    pub fn write_tdtr(&mut self, value: u32) {
        self.dlc = (value & 0xF) as u8;
    }
    pub fn write_tdlr(&mut self, value: u32) {
        self.data[0] = (value & 0xFF) as u8;
        self.data[1] = ((value >> 8) & 0xFF) as u8;
        self.data[2] = ((value >> 16) & 0xFF) as u8;
        self.data[3] = ((value >> 24) & 0xFF) as u8;
    }
    pub fn write_tdhr(&mut self, value: u32) {
        self.data[4] = (value & 0xFF) as u8;
        self.data[5] = ((value >> 8) & 0xFF) as u8;
        self.data[6] = ((value >> 16) & 0xFF) as u8;
        self.data[7] = ((value >> 24) & 0xFF) as u8;
    }

    pub fn store_message(&mut self, msg: CANMessage) {
        // TODO handle RTR and IDE
        self.id = msg.get_id();
        self.ide = true;
        self.rtr = false;
        self.dlc = msg.get_dlc();
        let data = msg.get_data();
        for i in 0..8 {
            self.data[i] = if i < data.len() { data[i] } else { 0 };
        }
    }
}

impl Into<CANMessage> for Mailbox {
    fn into(self) -> CANMessage {
        CANMessage::new(self.id, self.data, self.dlc)
    }
}
