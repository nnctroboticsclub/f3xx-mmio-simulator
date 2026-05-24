#![no_std]

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

pub mod context;
pub mod protocol;
pub mod segv_handler;
pub mod syscall;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        syscall::write(2, b"PANIC\n".as_ptr(), 6);
    }
    if let Some(loc) = info.location() {
        unsafe {
            syscall::write(2, loc.file().as_ptr(), loc.file().len());
            syscall::write(2, b":".as_ptr(), 1);
            write_hex(2, loc.line() as u64);
        }
    }
    unsafe {
        core::arch::asm!("mov rax, 60", "mov rdi, 1", "syscall", options(noreturn));
    }
}

struct BumpAllocator {
    heap: [u8; 1024 * 1024],
    pos: AtomicUsize,
}
unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        loop {
            let p = self.pos.load(Ordering::Relaxed);
            let start = (p + align - 1) & !(align - 1);
            let end = start + size;
            if end > self.heap.len() {
                unsafe {
                    syscall::write(2, b"OOM\n".as_ptr(), 4);
                }
                return core::ptr::null_mut();
            }
            if self
                .pos
                .compare_exchange_weak(p, end, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return self.heap.as_ptr().add(start) as *mut u8;
            }
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
    heap: [0; 1024 * 1024],
    pos: AtomicUsize::new(0),
};

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn init_mmio_simulator() {
    unsafe {
        syscall::write(2, b"1\n".as_ptr(), 2);
        let pid = syscall::getpid();
        syscall::fcntl(0, syscall::F_SETOWN, pid as i64);
        syscall::write(2, b"2\n".as_ptr(), 2);
        syscall::fcntl(
            0,
            syscall::F_SETFL,
            (syscall::O_ASYNC | syscall::O_NONBLOCK) as i64,
        );
        syscall::write(2, b"3\n".as_ptr(), 2);

        let sa_segv = syscall::SigAction {
            sa_handler: segv_handler::mmio_segv_handler as *const () as usize,
            sa_flags: syscall::SA_SIGINFO,
            sa_restorer: syscall::restore_rt as *const () as usize,
            sa_mask: 0,
        };
        syscall::sigaction(syscall::SIGSEGV, &sa_segv, core::ptr::null_mut());
        syscall::write(2, b"4\n".as_ptr(), 2);

        let sa_io = syscall::SigAction {
            sa_handler: segv_handler::sigio_handler as *const () as usize,
            sa_flags: syscall::SA_SIGINFO,
            sa_restorer: syscall::restore_rt as *const () as usize,
            sa_mask: 0,
        };
        syscall::sigaction(syscall::SIGIO, &sa_io, core::ptr::null_mut());
        syscall::write(2, b"5\n".as_ptr(), 2);
    }
}

pub unsafe fn write_hex(fd: i32, val: u64) {
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        let nibble = (val >> ((15 - i) * 4)) & 0xf;
        buf[i + 2] = if nibble < 10 {
            b'0' + nibble as u8
        } else {
            b'a' + (nibble - 10) as u8
        };
    }
    syscall::write(fd, buf.as_ptr(), 18);
}
