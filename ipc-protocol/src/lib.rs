#![no_std]

/// --- IPC Data Structures ---

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub cmd: u8,
    pub size: u8,
    pub seq: u16,
    pub _reserved: [u8; 4],
    pub address: u64,
    pub value: u64,
    pub timestamp: u64,
}

impl Request {
    pub fn new_ready() -> Self {
        Self {
            cmd: CMD_READY,
            size: 0,
            seq: 0,
            _reserved: [0; 4],
            address: 0,
            value: 0,
            timestamp: 0,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Response {
    pub resp_type: u8,
    pub seq: u16,
    pub _reserved: [u8; 5],
    pub value: u64,
}

impl Response {
    pub fn new_data(seq: u16, value: u64) -> Self {
        Self {
            resp_type: RESP_DATA,
            seq,
            _reserved: [0; 5],
            value,
        }
    }
}

/// --- Command & Response Codes ---

pub const CMD_READ: u8 = 1;
pub const CMD_WRITE: u8 = 2;
pub const CMD_TICK: u8 = 3;
pub const CMD_READY: u8 = 0xff;

pub const RESP_DATA: u8 = 0x81;
pub const RESP_INTERRUPT: u8 = 0x82;
pub const RESP_ERROR: u8 = 0x8F;
