use core::arch::global_asm;

pub unsafe fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 1,
        in("rdi") fd,
        in("rsi") buf,
        in("rdx") count,
        lateout("rax") ret,
        clobber_abi("system")
    );
    ret
}

pub unsafe fn read(fd: i32, buf: *mut u8, count: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 0,
        in("rdi") fd,
        in("rsi") buf,
        in("rdx") count,
        lateout("rax") ret,
        clobber_abi("system")
    );
    ret
}

pub unsafe fn fcntl(fd: i32, cmd: i32, arg: i64) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 72,
        in("rdi") fd,
        in("rsi") cmd,
        in("rdx") arg,
        lateout("rax") ret,
        clobber_abi("system")
    );
    ret
}

pub const F_SETOWN: i32 = 8;
pub const F_SETFL: i32 = 4;
pub const O_ASYNC: i32 = 0o20000;
pub const O_NONBLOCK: i32 = 0o4000;

pub unsafe fn sigaction(signum: i32, act: *const SigAction, oldact: *mut SigAction) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 13,
        in("rdi") signum,
        in("rsi") act,
        in("rdx") oldact,
        in("r10") 8, // size of sigset_t
        lateout("rax") ret,
        clobber_abi("system")
    );
    ret
}

#[repr(C)]
pub struct SigAction {
    pub sa_handler: usize,
    pub sa_flags: u64,
    pub sa_restorer: usize,
    pub sa_mask: u64,
}

pub const SA_SIGINFO: u64 = 0x00000004;
pub const SA_RESTORER: u64 = 0x04000000;
pub const SA_NODEFER: u64 = 0x40000000;

pub const SIGSEGV: i32 = 11;
pub const SIGIO: i32 = 29;

pub const SA_SIGINFO_FULL: u64 = SA_SIGINFO | SA_RESTORER;

pub unsafe fn getpid() -> i32 {
    let ret: i32;
    core::arch::asm!(
        "syscall",
        in("rax") 39,
        lateout("rax") ret,
        clobber_abi("system")
    );
    ret
}

global_asm!(
    ".global restore_rt",
    "restore_rt:",
    "mov rax, 15",
    "syscall"
);

extern "C" {
    pub fn restore_rt();
}
