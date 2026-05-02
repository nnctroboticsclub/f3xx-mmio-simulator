use std::{
    cell::RefCell,
    sync::{Arc, Mutex, OnceLock},
    thread,
};

use devconsole::{ChannelID, DCClient};
use tokio::{
    runtime::{Builder, Handle, Runtime},
    sync::mpsc,
};

use crate::runtime::get_runtime;

use super::DynMMIOHandler;

struct Device {
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

#[derive(Clone)]
pub struct DynDevice(Arc<Mutex<Device>>);

impl DynDevice {
    pub async fn new(url: &str) -> Self {
        Self(Arc::new(Mutex::new(Device::new(url).await)))
    }

    pub async fn open_channel(&self, channel_name: String) -> ChannelID {
        self.0
            .lock()
            .unwrap()
            .devconsole_client
            .open(channel_name)
            .await
            .unwrap()
    }

    pub async fn send(&self, channel_id: ChannelID, data: String) {
        self.0
            .lock()
            .unwrap()
            .devconsole_client
            .send(channel_id, data)
            .await
            .unwrap();
    }

    pub fn send_blocking(&self, channel_id: ChannelID, data: String) {
        get_runtime().block_on(self.send(channel_id, data))
    }

    pub async fn send_bin(&self, channel_id: ChannelID, data: Vec<u8>) {
        self.0
            .lock()
            .unwrap()
            .devconsole_client
            .send_bin(channel_id, data)
            .await
            .unwrap();
    }

    pub fn send_bin_blocking(&self, channel_id: ChannelID, data: Vec<u8>) {
        get_runtime().block_on(self.send_bin(channel_id, data))
    }

    pub async fn listen(
        &self,
        channel_id: ChannelID,
        tx: Option<mpsc::Sender<(ChannelID, String)>>,
        tx_bin: Option<mpsc::Sender<(ChannelID, Vec<u8>)>>,
    ) {
        self.0
            .lock()
            .unwrap()
            .devconsole_client
            .listen(channel_id, tx, tx_bin)
            .await
            .unwrap();
    }
}

pub struct Simulator {
    device: DynDevice,
    handlers: Vec<DynMMIOHandler>,
}

impl Simulator {
    pub fn new(device: DynDevice, handlers: Vec<DynMMIOHandler>) -> Self {
        Self { device, handlers }
    }

    pub fn lookup_handler(&mut self, address: usize) -> Option<&mut DynMMIOHandler> {
        self.handlers
            .iter_mut()
            .find(|handler| handler.contains(address))
    }
}

pub static SIMULATOR: OnceLock<Mutex<RefCell<Simulator>>> = OnceLock::new();
