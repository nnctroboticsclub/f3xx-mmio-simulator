use std::{
    cell::RefCell,
    sync::{Mutex, OnceLock},
};

use crate::mmio_handler::DynMMIOHandler;

#[derive(Debug)]
pub struct Simulator {
    handlers: Vec<DynMMIOHandler>,
}

impl Simulator {
    pub fn new(handlers: Vec<DynMMIOHandler>) -> Self {
        Self { handlers }
    }

    pub fn lookup_handler(&mut self, address: usize) -> Option<&mut DynMMIOHandler> {
        self.handlers
            .iter_mut()
            .find(|handler| handler.contains(address))
    }
}

pub static SIMULATOR: OnceLock<Mutex<RefCell<Simulator>>> = OnceLock::new();
