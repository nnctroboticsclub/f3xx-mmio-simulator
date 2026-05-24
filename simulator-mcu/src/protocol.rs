#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub cmd: u8,
    pub size: u8,
    pub _reserved: [u8; 6],
    pub address: u64,
    pub value: u64,
    pub timestamp: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Response {
    pub resp_type: u8,
    pub _reserved: [u8; 7],
    pub value: u64,
}

pub const CMD_READ: u8 = 1;
pub const CMD_WRITE: u8 = 2;
pub const CMD_TICK: u8 = 3;

pub const RESP_DATA: u8 = 0x81;
pub const RESP_INTERRUPT: u8 = 0x82;
pub const RESP_ERROR: u8 = 0x8F;
