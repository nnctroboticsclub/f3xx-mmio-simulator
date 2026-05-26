use crate::protocol::{Protocol, ProtocolHandler};
use crate::syscall;
use crate::{context::Context, syscall::exit};
use iced_x86::{Decoder, Mnemonic, OpKind};

struct DefaultProtocolHandler;
impl ProtocolHandler for DefaultProtocolHandler {
    unsafe fn inject_interrupt(context: *mut core::ffi::c_void, _irq_num: u64) {
        let ucontext = &mut *(context as *mut crate::context::UContext);
        let mcontext = &mut ucontext.uc_mcontext;

        let rsp = mcontext.gregs[15] as u64;
        let new_rsp = rsp - 8;
        let rip = mcontext.gregs[16] as u64;
        *(new_rsp as *mut u64) = rip;
        mcontext.gregs[15] = new_rsp as i64;

        // Placeholder jump
        // mcontext.gregs[16] = 0xdeadbeef;
    }
}

static PROTOCOL: Protocol<DefaultProtocolHandler> = Protocol::new();

/// # Safety
///
/// This function is marked as unsafe because it dereferences 'context' pointer, which is provided by non-Rust context.
#[no_mangle]
pub unsafe extern "C" fn sigio_handler(
    _signum: i32,
    _info: *mut core::ffi::c_void,
    context: *mut core::ffi::c_void,
) {
    PROTOCOL.sigio_handler(context);
}

pub extern "C" fn mmio_segv_handler(
    _signum: i32,
    _info: *mut core::ffi::c_void,
    context: *mut core::ffi::c_void,
) {
    let context_addr = context as u64;
    let rip_ptr = (context_addr + 168) as *mut u64;
    let rip = unsafe { *rip_ptr };

    let mut code_buf = [0u8; 15];
    unsafe {
        core::ptr::copy_nonoverlapping(rip as *const u8, code_buf.as_mut_ptr(), 15);
    }

    let mut decoder = Decoder::new(64, &code_buf, iced_x86::DecoderOptions::NONE);
    decoder.set_ip(rip);
    let inst = decoder.decode();

    let mut ctx =
        Context::new(unsafe { &mut *((context_addr + 40) as *mut crate::context::MContext) });

    match inst.mnemonic() {
        Mnemonic::Or => {
            let v1 = get_operand_value(&ctx, 0, &inst);
            let v2 = get_operand_value(&ctx, 1, &inst);
            set_operand_value(&mut ctx, &inst, 0, v1 | v2);
        }
        Mnemonic::Xor => {
            let v1 = get_operand_value(&ctx, 0, &inst);
            let v2 = get_operand_value(&ctx, 1, &inst);
            set_operand_value(&mut ctx, &inst, 0, v1 ^ v2);
        }
        Mnemonic::And => {
            let v1 = get_operand_value(&ctx, 0, &inst);
            let v2 = get_operand_value(&ctx, 1, &inst);
            set_operand_value(&mut ctx, &inst, 0, v1 & v2);
        }
        Mnemonic::Mov => {
            let v = get_operand_value(&ctx, 1, &inst);
            set_operand_value(&mut ctx, &inst, 0, v);
        }
        Mnemonic::Test => {
            let v1 = get_operand_value(&ctx, 0, &inst);
            let v2 = get_operand_value(&ctx, 1, &inst);
            let res = v1 & v2;

            let flags =
                unsafe { &mut (*((context_addr + 40) as *mut crate::context::MContext)).gregs[17] };
            *flags &= !((1 << 0) | (1 << 6) | (1 << 7) | (1 << 2) | (1 << 11));
            if (res & 0x80000000) != 0 {
                *flags |= 1 << 7;
            }
            if res == 0 {
                *flags |= 1 << 6;
            }
        }
        _ => unsafe {
            syscall::write(2, b"Unknown Mnemonic: ".as_ptr(), 18);
            crate::write_hex(2, inst.mnemonic() as u64);
            exit(1);
        },
    }

    unsafe {
        *rip_ptr += inst.len() as u64;
    }
}

fn get_operand_value(ctx: &Context, op_idx: u32, inst: &iced_x86::Instruction) -> u64 {
    match inst.op_kind(op_idx) {
        OpKind::Register => ctx
            .get_register_value(inst.op_register(op_idx))
            .unwrap_or(0),
        OpKind::Immediate8 => inst.immediate8() as u64,
        OpKind::Immediate8_2nd => inst.immediate8_2nd() as u64,
        OpKind::Immediate16 => inst.immediate16() as u64,
        OpKind::Immediate32 => inst.immediate32() as u64,
        OpKind::Immediate64 => inst.immediate64(),
        OpKind::Immediate8to16 => inst.immediate8to16() as u64,
        OpKind::Immediate8to32 => inst.immediate8to32() as u64,
        OpKind::Immediate8to64 => inst.immediate8to64() as u64,
        OpKind::Immediate32to64 => inst.immediate32to64() as u64,
        OpKind::Memory => {
            let addr = inst
                .virtual_address(op_idx, 0, |reg, _, _| ctx.get_register_value(reg))
                .unwrap_or(0);
            PROTOCOL.read(addr, inst.memory_size())
        }
        _ => 0,
    }
}

fn set_operand_value(ctx: &mut Context, inst: &iced_x86::Instruction, op_idx: u32, value: u64) {
    match inst.op_kind(op_idx) {
        OpKind::Register => ctx.write_register(inst.op_register(op_idx), value),
        OpKind::Memory => {
            let addr = inst
                .virtual_address(op_idx, 0, |reg, _, _| ctx.get_register_value(reg))
                .unwrap_or(0);
            PROTOCOL.write(addr, value, inst.memory_size());
        }
        _ => {}
    }
}
