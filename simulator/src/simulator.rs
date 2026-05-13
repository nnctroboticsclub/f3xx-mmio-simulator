use std::{
    cell::RefCell,
    sync::{mpsc as std_mpsc, Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::sync::mpsc as tokio_mpsc;

use devconsole::{ChannelID, DCClient};

use crate::vector_table::VectorTablePtr;

use super::DynMMIOHandler;

enum ThreadRequest {
    OpenChannel(String),
    // Send(ChannelID, String),
    SendBin(ChannelID, Vec<u8>),
    Listen(
        ChannelID,
        Option<std_mpsc::Sender<(ChannelID, String)>>,
        Option<std_mpsc::Sender<(ChannelID, Vec<u8>)>>,
    ),
}

enum ThreadResponse {
    OpenChannelResult(ChannelID),
}

struct Device {
    vector_table: VectorTablePtr,
    active_interrupt: Option<u32>,

    request_tx: std_mpsc::Sender<ThreadRequest>,
    response_rx: std_mpsc::Receiver<ThreadResponse>,
}

async fn copy_mpsc_to_std<T: Send + 'static>(
    std_tx: std_mpsc::Sender<T>,
    mut tokio_rx: tokio_mpsc::Receiver<T>,
) {
    while let Some(value) = tokio_rx.recv().await {
        if std_tx.send(value).is_err() {
            break;
        }
    }
}

async fn mpsc_rx_to_tokio<T: Send + 'static>(std_tx: std_mpsc::Sender<T>) -> tokio_mpsc::Sender<T> {
    type TokioTx<T> = tokio_mpsc::Sender<T>;
    type TokioRx<T> = tokio_mpsc::Receiver<T>;

    let (tokio_tx, tokio_rx): (TokioTx<T>, TokioRx<T>) = tokio_mpsc::channel(100);
    tokio::spawn(copy_mpsc_to_std(std_tx, tokio_rx));

    tokio_tx
}

#[derive(Clone)]
pub struct DynDevice(Arc<Mutex<Device>>);

impl DynDevice {
    pub async fn new(url: &str) -> Self {
        let (request_tx, request_rx) = std_mpsc::channel();
        let (response_tx, response_rx) = std_mpsc::channel();
        let dev = Device {
            vector_table: VectorTablePtr::new_null(),
            active_interrupt: None,
            request_tx,
            response_rx,
        };
        let obj = Self(Arc::new(Mutex::new(dev)));
        {
            let url = url.to_string();
            std::thread::Builder::new()
                .name("DCClientThread".to_string())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(Self::task(url, request_rx, response_tx));
                })
                .expect("Failed to spawn DCClient thread");
        }
        obj
    }

    async fn task(
        url: String,
        request_rx: std_mpsc::Receiver<ThreadRequest>,
        response_tx: std_mpsc::Sender<ThreadResponse>,
    ) {
        let mut dc_client = match DCClient::new(&url).await {
            Ok(client) => Some(client),
            Err(_err) => None,
        };

        loop {
            match request_rx.recv() {
                Ok(ThreadRequest::OpenChannel(channel_name)) => {
                    let channel_id = if let Some(client) = dc_client.as_mut() {
                        match client.open(channel_name).await {
                            Ok(channel_id) => channel_id,
                            Err(_err) => {
                                dc_client = None;
                                0
                            }
                        }
                    } else {
                        0
                    };
                    response_tx
                        .send(ThreadResponse::OpenChannelResult(channel_id))
                        .unwrap();
                }
                /* Ok(ThreadRequest::Send(channel_id, data)) => {
                    dc_client.send(channel_id, data).await.unwrap();
                } */
                Ok(ThreadRequest::SendBin(channel_id, data)) => {
                    if let Some(client) = dc_client.as_mut() {
                        if let Err(_err) = client.send_bin(channel_id, data).await {
                            dc_client = None;
                        }
                    }
                }
                Ok(ThreadRequest::Listen(channel_id, s_tx_txt, s_tx_bin)) => {
                    if dc_client.is_none() {
                        continue;
                    }

                    let t_tx_txt = if let Some(s_tx_txt) = s_tx_txt {
                        Some(mpsc_rx_to_tokio(s_tx_txt).await)
                    } else {
                        None
                    };
                    let t_tx_bin = if let Some(s_tx_bin) = s_tx_bin {
                        Some(mpsc_rx_to_tokio(s_tx_bin).await)
                    } else {
                        None
                    };

                    if let Some(client) = dc_client.as_mut() {
                        if let Err(_err) = client.listen(channel_id, t_tx_txt, t_tx_bin).await {
                            dc_client = None;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }

    pub fn open_channel(&self, channel_name: String) -> ChannelID {
        self.0
            .lock()
            .unwrap()
            .request_tx
            .send(ThreadRequest::OpenChannel(channel_name))
            .unwrap();

        let response = self
            .0
            .lock()
            .unwrap()
            .response_rx
            .recv_timeout(Duration::from_secs(5));

        match response {
            Ok(ThreadResponse::OpenChannelResult(channel_id)) => channel_id,
            Err(_) => 0,
        }
    }

    /* pub fn _send(&self, channel_id: ChannelID, data: String) {
        self.0
            .lock()
            .unwrap()
            .request_tx
            .send(ThreadRequest::Send(channel_id, data))
            .unwrap();
    } */
    pub fn send_bin(&self, channel_id: ChannelID, data: Vec<u8>) {
        self.0
            .lock()
            .unwrap()
            .request_tx
            .send(ThreadRequest::SendBin(channel_id, data))
            .unwrap();
    }
    pub fn listen(
        &self,
        channel_id: ChannelID,
        tx: Option<std_mpsc::Sender<(ChannelID, String)>>,
        tx_bin: Option<std_mpsc::Sender<(ChannelID, Vec<u8>)>>,
    ) {
        self.0
            .lock()
            .unwrap()
            .request_tx
            .send(ThreadRequest::Listen(channel_id, tx, tx_bin))
            .unwrap();
    }

    pub fn get_vector_table(&self) -> VectorTablePtr {
        self.0.lock().unwrap().vector_table
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
            unsafe { std::mem::transmute::<*const (), extern "C" fn()>(handler) }
        } else {
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
    handlers: Vec<DynMMIOHandler>,
}

impl Simulator {
    pub fn new(_device: DynDevice, handlers: Vec<DynMMIOHandler>) -> Self {
        Self { handlers }
    }

    pub fn lookup_handler(&mut self, address: usize) -> Option<&mut DynMMIOHandler> {
        self.handlers
            .iter_mut()
            .find(|handler| handler.contains(address))
    }
}

pub static SIMULATOR: OnceLock<Mutex<RefCell<Simulator>>> = OnceLock::new();
