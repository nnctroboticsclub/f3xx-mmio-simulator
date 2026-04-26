use std::{
    cell::RefCell,
    sync::{Mutex, OnceLock},
};

use iced_x86::{
    Decoder, Formatter, GasFormatter, Instruction, InstructionInfoFactory, Mnemonic, OpKind,
    Register,
};
use nix::{
    libc::{self, mcontext_t},
    sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
};

use crate::{
    bridge_region::BridgeRegion,
    context::{iced_register_to_libc_reg, Context},
    flash_region::FlashRegion,
    mmio_handler::DynMMIOHandler,
    rcc_region::RCCRegion,
    segv_handler::mmio_segv_handler,
    simulator::{Simulator, SIMULATOR},
};

mod bridge_region;
mod context;
mod flash_region;
mod mem_handler;
mod mmio_handler;
mod not_implemented_handler;
mod rcc_region;
mod segv_handler;
mod simulator;
mod vector_table;

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        fn init_mmio_simulator();
    }
}

pub extern "C" fn init_mmio_simulator() {
    SIMULATOR
        .set(Mutex::new({
            let mut handlers = Vec::<DynMMIOHandler>::new();
            handlers.push(FlashRegion::new_boxed(0x4002_2000));
            handlers.push(RCCRegion::new_boxed(0x4002_1000));
            handlers.push(BridgeRegion::new_boxed(0xabcd_0000));

            let sim = Simulator::new(handlers);
            RefCell::new(sim)
        }))
        .expect("Failed to initialize the MMIO simulator");
    let sa = SigAction::new(
        SigHandler::SigAction(mmio_segv_handler),
        SaFlags::SA_NODEFER | SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );
    unsafe {
        sigaction(Signal::SIGSEGV, &sa).expect("Failed to register the mmio signal handler");
    }
}
