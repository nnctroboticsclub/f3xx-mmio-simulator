pub struct CANMessage {
    id: u32,
    data: [u8; 8],
    dlc: u8,
}

impl CANMessage {
    pub fn new(id: u32, data: [u8; 8], dlc: u8) -> Self {
        Self { id, data, dlc }
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }
    pub fn get_data(&self) -> &[u8; 8] {
        &self.data
    }

    pub fn get_dlc(&self) -> u8 {
        self.dlc
    }
}

impl Into<Vec<u8>> for CANMessage {
    fn into(self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.id.to_le_bytes());
        v.push(self.dlc);
        v.extend_from_slice(&self.data);
        return v;
    }
}

impl From<Vec<u8>> for CANMessage {
    fn from(v: Vec<u8>) -> Self {
        let id = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);
        let dlc = v[4];
        let mut data = [0u8; 8];
        data.copy_from_slice(&v[5..13]);
        return Self { id, data, dlc };
    }
}
