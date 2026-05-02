use std::{
    cell::RefCell,
    sync::{Arc, Mutex, OnceLock},
};

use devconsole::DCClient;

use super::DynMMIOHandler;

pub struct Device {
    devconsole_client: DCClient,
}

impl Device {
    pub async fn new(url: &str) -> Self {
        Self {
            devconsole_client: DCClient::new(url)
                .await
                .expect("Failed to connect to the devconsole server"),
        }
    }
}

pub struct Simulator {
    device: Arc<Device>,
    handlers: Vec<DynMMIOHandler>,
}

impl Simulator {
    pub fn new(device: Arc<Device>, handlers: Vec<DynMMIOHandler>) -> Self {
        Self { device, handlers }
    }

    pub fn lookup_handler(&mut self, address: usize) -> Option<&mut DynMMIOHandler> {
        self.handlers
            .iter_mut()
            .find(|handler| handler.contains(address))
    }
}

pub static SIMULATOR: OnceLock<Mutex<RefCell<Simulator>>> = OnceLock::new();
