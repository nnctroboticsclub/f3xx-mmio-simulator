use iced_x86::{
    Decoder, Formatter, GasFormatter, Instruction, InstructionInfoFactory, Mnemonic, OpKind,
    Register,
};
use nix::libc::{self, mcontext_t};

use crate::{
    context::{iced_register_to_libc_reg, Context},
    runtime::get_runtime,
    simulator::SIMULATOR,
};

fn reraise_as_native_segv() {
    unsafe {
        libc::signal(libc::SIGSEGV, libc::SIG_DFL);
        libc::raise(libc::SIGSEGV);
        libc::exit(1);
    }
}

fn diagnose_inst(mcontext: &mcontext_t, inst: Instruction) {
    let mut info_factory = InstructionInfoFactory::new();
    let info = info_factory.info(&inst);

    let mut inst_formatter = GasFormatter::new();

    let get_register_value = |reg, _index, _size| match reg {
        Register::ES | Register::CS | Register::SS | Register::DS => Some(0),
        _ => Some(mcontext.gregs[reg as usize] as u64),
    };

    let out = {
        let mut out = String::new();
        inst_formatter.format(&inst, &mut out);
        out
    };
    println!("@PC + 0x00: {:?}: {out}", inst.mnemonic());
    for i in 0..inst.op_count() {
        let kind = inst.op_kind(i);
        let access = info.op_access(i);
        match kind {
            OpKind::Memory => {
                let va = inst.virtual_address(i, 0, get_register_value);
                if va.is_none() {
                    eprintln!("Failed to compute the virtual address");
                    reraise_as_native_segv();
                }
                let va = va.unwrap() as usize;
                println!("- Memory access at address {va:#x} as {access:?}");
            }
            OpKind::Register => {
                let reg = inst.op_register(i);
                if let Some(reg) = iced_register_to_libc_reg(reg) {
                    let value = mcontext.gregs[reg as usize];
                    println!("- Register access: {reg:?} with value {value:#x} as {access:?}");
                } else {
                    println!("- Register access: {reg:?} as {access:?}");
                }
            }
            _ => {
                println!("- Other operand: {kind:?} as {access:?}");
            }
        }
    }
}

async fn segv_handler(
    _signum: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let address = unsafe { info.read().si_addr() as usize };
    let ucontext = unsafe { &mut *(context as *mut libc::ucontext_t) };
    let mcontext = &mut ucontext.uc_mcontext;
    let pc = mcontext.gregs[libc::REG_RIP as usize] as *const u8;

    let simulator = SIMULATOR
        .get()
        .expect("Simulator not initialized")
        .lock()
        .unwrap();
    let mut simulator = simulator.borrow_mut();

    let handler = {
        let handler = simulator.lookup_handler(address);
        if handler.is_none() {
            eprintln!(
                "Segmentation fault at address {address:#x} (PC: {pc:?}) with no handler found"
            );
            reraise_as_native_segv();
        }
        handler.unwrap()
    };

    let mut decoder = Decoder::new(
        64,
        unsafe { std::slice::from_raw_parts(pc, 15) },
        iced_x86::DecoderOptions::NONE,
    );
    let inst = decoder.decode();

    let mut ctx = Context::new(mcontext, handler);

    match inst.mnemonic() {
        Mnemonic::Or => {
            let value = ctx.get_operand_value(0, &inst).unwrap();
            let operand_value = ctx.get_operand_value(1, &inst).unwrap();
            let new_value = value | operand_value;
            ctx.set_operand_value(inst, 0, new_value);
        }
        Mnemonic::Xor => {
            let value = ctx.get_operand_value(0, &inst).unwrap();
            let operand_value = ctx.get_operand_value(1, &inst).unwrap();
            let new_value = value ^ operand_value;
            ctx.set_operand_value(inst, 0, new_value);
        }
        Mnemonic::And => {
            let op0 = ctx.get_operand_value(0, &inst).unwrap();
            let op1 = ctx.get_operand_value(1, &inst).unwrap();
            let new_value = op0 & op1;
            ctx.set_operand_value(inst, 0, new_value);
        }
        Mnemonic::Mov => {
            let value = ctx.get_operand_value(1, &inst).unwrap();
            ctx.set_operand_value(inst, 0, value);
        }
        Mnemonic::Test => {
            let value = ctx.get_operand_value(0, &inst).unwrap();
            let operand_value = ctx.get_operand_value(1, &inst).unwrap();
            let new_value = value & operand_value;

            let flags = &mut mcontext.gregs[libc::REG_EFL as usize];
            *flags &= !(1 << 0); // CF
            *flags &= !(1 << 6); // ZF
            *flags &= !(1 << 7); // SF
            *flags &= !(1 << 2); // PF
            *flags &= !!(1 << 11); // OF
            if (new_value & 0x80) != 0 {
                *flags |= 1 << 7; // SF
            }
            if new_value == 0 {
                *flags |= 1 << 6; // ZF
            }
            let mut parity = 0;
            for i in 0..8 {
                if (new_value & (1 << i)) != 0 {
                    parity += 1;
                }
            }
            if parity % 2 == 0 {
                *flags |= 1 << 2; // PF
            }
        }
        _ => {
            println!(
                "Unsupported instruction at PC {pc:?}: {:?}",
                inst.mnemonic()
            );
            diagnose_inst(mcontext, inst);
            reraise_as_native_segv();
        }
    }

    mcontext.gregs[libc::REG_RIP as usize] += inst.len() as i64;
}

pub extern "C" fn mmio_segv_handler(
    signum: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let future = segv_handler(signum, info, context);
    get_runtime().block_on(future);
}
