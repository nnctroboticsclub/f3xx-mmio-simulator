use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use tokio::runtime::Builder;

use crate::{
    regions::{
        BXCanRegion, BasicTimerRegion, BridgeRegion, DynMMIOHandler, FlashRegion, GPIOPort,
        GPIORegion, NVICRegion, RCCRegion, SCBRegion, UARTRegion,
    },
    segv_handler::mmio_segv_handler,
    simulator::{Device, Simulator, SIMULATOR},
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
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");
    let dev = rt.block_on(Device::new("ws://localhost:9001"));
    let dev = Arc::new(dev);

    let mut handlers = Vec::<DynMMIOHandler>::new();
    handlers.push(FlashRegion::new_boxed(dev.clone(), 0x4002_2000));
    handlers.push(RCCRegion::new_boxed(dev.clone(), 0x4002_1000));
    handlers.push(BridgeRegion::new_boxed(dev.clone(), 0xabcd_0000));
    handlers.push(GPIORegion::new_gpioa(dev.clone(), 0x48000000));
    handlers.push(GPIORegion::new_gpiob(dev.clone(), 0x48000400));
    handlers.push(GPIORegion::new_boxed(dev.clone(), 0x48000800, GPIOPort::C));
    handlers.push(GPIORegion::new_boxed(dev.clone(), 0x48000C00, GPIOPort::D));
    handlers.push(GPIORegion::new_boxed(dev.clone(), 0x48001400, GPIOPort::F));
    handlers.push(UARTRegion::new_boxed(dev.clone(), 0x40004400, 2));
    handlers.push(BXCanRegion::new_boxed(dev.clone(), 0x40006400));
    handlers.push(SCBRegion::new_boxed(dev.clone(), 0xE000_ED00));
    handlers.push(NVICRegion::new_boxed(dev.clone(), 0xE000_E100));
    handlers.push(BasicTimerRegion::new_boxed(dev.clone(), 0x40001000));

    let sim = Simulator::new(dev, handlers);
    let sim = RefCell::new(sim);

    match SIMULATOR.set(Mutex::new(sim)) {
        Ok(_) => (),
        Err(_) => panic!("Simulator is already initialized"),
    }

    let sa = SigAction::new(
        SigHandler::SigAction(mmio_segv_handler),
        SaFlags::SA_NODEFER | SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );
    unsafe {
        sigaction(Signal::SIGSEGV, &sa).expect("Failed to register the mmio signal handler");
    }
}
