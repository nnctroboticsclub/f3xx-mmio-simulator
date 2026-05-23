use iced_x86::{Decoder, Mnemonic};
use nix::libc;

use crate::{
    context::Context,
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
