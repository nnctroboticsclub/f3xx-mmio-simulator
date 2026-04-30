use std::{cell::RefCell, sync::Mutex};

use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};

use crate::{
    regions::BXCanRegion,
    regions::BasicTimerRegion,
    regions::BridgeRegion,
    regions::DynMMIOHandler,
    regions::FlashRegion,
    regions::NVICRegion,
    regions::RCCRegion,
    regions::SCBRegion,
    regions::UARTRegion,
    regions::{GPIOPort, GPIORegion},
    segv_handler::mmio_segv_handler,
    simulator::{Simulator, SIMULATOR},
};

mod context;
mod regions;
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
            handlers.push(GPIORegion::new_gpioa(0x48000000));
            handlers.push(GPIORegion::new_gpiob(0x48000400));
            handlers.push(GPIORegion::new_boxed(0x48000800, GPIOPort::C));
            handlers.push(GPIORegion::new_boxed(0x48000C00, GPIOPort::D));
            handlers.push(GPIORegion::new_boxed(0x48001400, GPIOPort::F));
            handlers.push(UARTRegion::new_boxed(0x40004400, 2));
            handlers.push(BXCanRegion::new_boxed(0x40006400));
            handlers.push(SCBRegion::new_boxed(0xE000_ED00));
            handlers.push(NVICRegion::new_boxed(0xE000_E100));
            handlers.push(BasicTimerRegion::new_boxed(0x40001000));

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
