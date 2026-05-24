use iced_x86::Register;

#[repr(C)]
pub struct MContext {
    pub gregs: [i64; 23],
    pub fpregs: usize,
    pub __reserved1: [u64; 8],
}

#[repr(C)]
pub struct UContext {
    pub uc_flags: u64,
    pub uc_link: usize,
    pub uc_stack: [u64; 3],
    pub uc_mcontext: MContext,
    pub uc_sigmask: [u64; 16],
}

pub fn iced_register_to_libc_reg(reg: Register) -> Option<usize> {
    match reg {
        Register::R8 | Register::R8D | Register::R8W | Register::R8L => Some(0),
        Register::R9 | Register::R9D | Register::R9W | Register::R9L => Some(1),
        Register::R10 | Register::R10D | Register::R10W | Register::R10L => Some(2),
        Register::R11 | Register::R11D | Register::R11W | Register::R11L => Some(3),
        Register::R12 | Register::R12D | Register::R12W | Register::R12L => Some(4),
        Register::R13 | Register::R13D | Register::R13W | Register::R13L => Some(5),
        Register::R14 | Register::R14D | Register::R14W | Register::R14L => Some(6),
        Register::R15 | Register::R15D | Register::R15W | Register::R15L => Some(7),
        Register::RDI | Register::EDI | Register::DI | Register::DIL => Some(8),
        Register::RSI | Register::ESI | Register::SI | Register::SIL => Some(9),
        Register::RBP | Register::EBP | Register::BP | Register::BPL => Some(10),
        Register::RBX | Register::EBX | Register::BX | Register::BL => Some(11),
        Register::RDX | Register::EDX | Register::DX | Register::DL | Register::DH => Some(12),
        Register::RAX | Register::EAX | Register::AX | Register::AL | Register::AH => Some(13),
        Register::RCX | Register::ECX | Register::CX | Register::CL | Register::CH => Some(14),
        Register::RSP | Register::ESP | Register::SP | Register::SPL => Some(15),
        Register::RIP => Some(16),
        _ => None,
    }
}

pub struct Context<'a> {
    pub mcontext: &'a mut MContext,
}

impl<'a> Context<'a> {
    pub fn new(mcontext: &'a mut MContext) -> Self {
        Self { mcontext }
    }

    pub fn get_register_value(&self, reg: Register) -> Option<u64> {
        if let Some(idx) = iced_register_to_libc_reg(reg) {
            return Some(self.mcontext.gregs[idx] as u64);
        }
        match reg {
            Register::ES | Register::CS | Register::SS | Register::DS => Some(0),
            _ => None,
        }
    }

    pub fn write_register(&mut self, reg: Register, value: u64) {
        if let Some(idx) = iced_register_to_libc_reg(reg) {
            self.mcontext.gregs[idx] = value as i64;
        }
    }
}
