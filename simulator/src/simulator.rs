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

use crate::{
    runtime::get_runtime,
    vector_table::{VectorTable, VectorTablePtr},
};

use super::DynMMIOHandler;

struct Device {
    devconsole_client: DCClient,
    vector_table: VectorTablePtr,
    active_interrupt: Option<u32>,
}

impl Device {
    pub async fn new(url: &str) -> Self {
        Self {
            devconsole_client: DCClient::new(url)
                .await
                .expect("Failed to connect to the devconsole server"),
            vector_table: VectorTablePtr::new_null(),
            active_interrupt: None,
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

    pub fn get_vector_table(&self) -> VectorTablePtr {
        self.0.lock().unwrap().vector_table.clone()
    }

    pub fn set_vector_table(&self, vector_table: VectorTablePtr) {
        self.0.lock().unwrap().vector_table = vector_table;
    }

    pub fn fire_interrupt(&self, irqn: u32) {
        let handler: extern "C" fn() = if let Some(handler) = self
            .0
            .lock()
            .unwrap()
            .vector_table
            .try_get_handler(irqn as usize)
        {
            unsafe { std::mem::transmute(handler) }
        } else {
            println!("Vector table is not set");
            println!(
                "vector_table: {:?}",
                self.0.lock().unwrap().vector_table.as_ptr()
            );
            return;
        };

        self.0.lock().unwrap().active_interrupt = Some(irqn);
        handler();
        self.0.lock().unwrap().active_interrupt = None;
    }

    pub fn get_active_interrupt(&self) -> Option<u32> {
        self.0.lock().unwrap().active_interrupt
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
