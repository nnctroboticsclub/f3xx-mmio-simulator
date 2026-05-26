use crate::syscall::{read, write};
use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use iced_x86::MemorySize;
use ipc_protocol::{Request, Response, CMD_READ, CMD_WRITE};
use ipc_protocol::{RESP_DATA, RESP_INTERRUPT};

pub trait ProtocolHandler {
    /// # Safety
    /// This function should be implemented with the assumption that it can be called from a signal handler context.
    unsafe fn inject_interrupt(context: *mut core::ffi::c_void, irq_num: u64);
}

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

pub fn recv_response() -> Option<Response> {
    let mut resp = Response {
        resp_type: 0,
        seq: 0,
        _reserved: [0; 5],
        value: 0,
    };

    let read_bytes = unsafe {
        read(
            0,
            &mut resp as *mut Response as *mut u8,
            core::mem::size_of::<Response>(),
        )
    };

    if read_bytes == core::mem::size_of::<Response>() as isize {
        Some(resp)
    } else {
        None
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

fn send_read(addr: u64, size: MemorySize, seq: u16) {
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

fn send_write(addr: u64, size: MemorySize, value: u64, seq: u16) {
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

pub struct Protocol<T: ProtocolHandler> {
    protocol_handler: core::marker::PhantomData<T>,

    waiting_for_data: AtomicBool,
    last_read_value: AtomicU64,
    next_seq: AtomicU16,
    current_wait_seq: AtomicU16,
}

impl<T: ProtocolHandler> Protocol<T> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            protocol_handler: core::marker::PhantomData,
            waiting_for_data: AtomicBool::new(false),
            last_read_value: AtomicU64::new(0),
            next_seq: AtomicU16::new(1),
            current_wait_seq: AtomicU16::new(0),
        }
    }

    /// # Safety
    /// The caller must ensure that 'context' is a valid pointer.
    pub unsafe fn sigio_handler(&self, context: *mut core::ffi::c_void) {
        while let Some(resp) = recv_response() {
            match resp.resp_type {
                RESP_DATA if resp.seq == self.current_wait_seq.load(Ordering::SeqCst) => {
                    self.last_read_value.store(resp.value, Ordering::SeqCst);
                    self.waiting_for_data.store(false, Ordering::SeqCst);
                }
                RESP_INTERRUPT => unsafe {
                    T::inject_interrupt(context, resp.value);
                },
                _ => {}
            }
        }
    }

    pub fn read(&self, addr: u64, size: MemorySize) -> u64 {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);

        self.current_wait_seq.store(seq, Ordering::SeqCst);
        self.waiting_for_data.store(true, Ordering::SeqCst);

        send_read(addr, size, seq);

        while self.waiting_for_data.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }

        self.last_read_value.load(Ordering::SeqCst)
    }

    pub fn write(&self, addr: u64, value: u64, size: MemorySize) {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        send_write(addr, size, value, seq);
    }
}
