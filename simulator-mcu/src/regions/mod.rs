mod basic_timer_region;
mod bridge_region;
mod bxcan_region;
mod flash_region;
mod gpio_region;

mod mmio_handler;
mod nvic_region;
mod rcc_region;
mod scb_region;
mod uart_region;

use mmio_handler::MmioHandler;

pub use basic_timer_region::BasicTimerRegion;
pub use bridge_region::BridgeRegion;
pub use bxcan_region::BXCanRegion;
pub use flash_region::FlashRegion;
pub use gpio_region::{GPIOPort, GPIORegion};
pub use mmio_handler::DynMMIOHandler;
pub use nvic_region::NVICRegion;
pub use rcc_region::RCCRegion;
pub use scb_region::SCBRegion;
pub use uart_region::UARTRegion;
