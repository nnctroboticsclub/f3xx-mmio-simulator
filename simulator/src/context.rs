use iced_x86::{Instruction, OpKind, Register};
use nix::libc::{self, mcontext_t};

use super::DynMMIOHandler;

pub fn iced_register_to_libc_reg(reg: Register) -> Option<libc::c_int> {
    match reg {
        Register::RAX | Register::EAX => Some(libc::REG_RAX),
        Register::RCX | Register::ECX => Some(libc::REG_RCX),
        Register::RDX | Register::EDX => Some(libc::REG_RDX),
        Register::RBX | Register::EBX => Some(libc::REG_RBX),
        Register::RSP | Register::ESP => Some(libc::REG_RSP),
        Register::RBP | Register::EBP => Some(libc::REG_RBP),
        Register::RSI | Register::ESI => Some(libc::REG_RSI),
        Register::RDI | Register::EDI => Some(libc::REG_RDI),
        Register::R8D | Register::R8 => Some(libc::REG_R8),
        Register::R9D | Register::R9 => Some(libc::REG_R9),
        Register::R10D | Register::R10 => Some(libc::REG_R10),
        Register::R11D | Register::R11 => Some(libc::REG_R11),
        Register::R12D | Register::R12 => Some(libc::REG_R12),
        Register::R13D | Register::R13 => Some(libc::REG_R13),
        Register::R14D | Register::R14 => Some(libc::REG_R14),
        Register::R15D | Register::R15 => Some(libc::REG_R15),
        _ => None,
    }
}

pub struct Context<'a> {
    mcontext: &'a mut mcontext_t,
    region: &'a mut DynMMIOHandler,
}

impl<'a> Context<'a> {
    pub fn new(mcontext: &'a mut mcontext_t, region: &'a mut DynMMIOHandler) -> Self {
        Self { mcontext, region }
    }

    fn get_register_value(&self, reg: Register) -> Option<u64> {
        if let Some(libc_reg) = iced_register_to_libc_reg(reg) {
            return Some(self.mcontext.gregs[libc_reg as usize] as u64);
        }
        match reg {
            Register::ES | Register::CS | Register::SS | Register::DS => Some(0),
            _ => {
                panic!("Unsupported register access: {:?}", reg);
            }
        }
    }
    fn write_register(&mut self, reg: Register, value: u64) {
        if let Some(libc_reg) = iced_register_to_libc_reg(reg) {
            self.mcontext.gregs[libc_reg as usize] = value as i64;
        } else {
            panic!("Unsupported register for writing: {:?}", reg);
        }
    }

    fn write_u32_memory(&mut self, address: usize, value: u32) {
        self.region.write(address, value);
    }

    fn read_u32_memory(&self, address: usize) -> u32 {
        self.region.read(address)
    }

    pub fn set_operand_value(&mut self, inst: Instruction, op_index: u32, value: u32) {
        match inst.op_kind(op_index) {
            OpKind::Register => {
                let reg = inst.op_register(op_index);
                self.write_register(reg, value as u64);
            }
            OpKind::Memory => {
                let address = inst
                    .virtual_address(op_index, 0, |reg, _, _| self.get_register_value(reg))
                    .unwrap() as usize;
                self.write_u32_memory(address, value);
            }
            _ => {
                panic!(
                    "Unsupported operand kind for writing: {:?}",
                    inst.op_kind(op_index)
                );
            }
        }
    }

    pub fn get_operand_value(&self, index: u32, inst: &Instruction) -> Option<u64> {
        if let Ok(imm) = inst.try_immediate(index) {
            return Some(imm);
        }

        match inst.op_kind(index) {
            OpKind::Register => {
                let reg = inst.op_register(index);
                self.get_register_value(reg)
            }
            OpKind::Memory => {
                let address = inst
                    .virtual_address(index, 0, |reg, _, _| self.get_register_value(reg))
                    .unwrap() as usize;

                Some(self.read_u32_memory(address) as u64)
            }
            _ => {
                println!(
                    "Unsupported operand kind for operand {index}: {:?}",
                    inst.op_kind(index)
                );
                None
            }
        }
    }
}
