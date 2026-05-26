use iced_x86::MemorySize;
use ipc_protocol::{Request, CMD_READ, CMD_WRITE};

use crate::syscall::write;

fn get_memory_size(ms: iced_x86::MemorySize) -> u8 {
    match ms {
        iced_x86::MemorySize::UInt8 | iced_x86::MemorySize::Int8 => 1,
        iced_x86::MemorySize::UInt16 | iced_x86::MemorySize::Int16 => 2,
        iced_x86::MemorySize::UInt32 | iced_x86::MemorySize::Int32 => 4,
        iced_x86::MemorySize::UInt64 | iced_x86::MemorySize::Int64 => 8,
        _ => 4,
    }
}

fn send_request(request: &Request) {
    unsafe {
        write(
            1,
            request as *const Request as *const u8,
            core::mem::size_of::<Request>(),
        );
    }
}

pub fn send_ready() {
    let ready = ipc_protocol::Request {
        cmd: ipc_protocol::CMD_READY,
        size: 0,
        seq: 0,
        _reserved: [0; 4],
        address: 0,
        value: 0,
        timestamp: 0,
    };
    send_request(&ready);
}

pub fn send_read(addr: u64, size: MemorySize, seq: u16) {
    let req = Request {
        cmd: CMD_READ,
        size: get_memory_size(size),
        seq,
        _reserved: [0; 4],
        address: addr,
        value: 0,
        timestamp: 0,
    };

    send_request(&req);
}

pub fn send_write(addr: u64, size: MemorySize, value: u64, seq: u16) {
    let req = Request {
        cmd: CMD_WRITE,
        size: get_memory_size(size),
        seq,
        _reserved: [0; 4],
        address: addr,
        value,
        timestamp: 0,
    };

    send_request(&req);
}
