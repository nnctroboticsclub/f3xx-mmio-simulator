use crate::regions::{
    BXCanRegion, BasicTimerRegion, BridgeRegion, DynMMIOHandler, FlashRegion, GPIOPort, GPIORegion,
    NVICRegion, RCCRegion, SCBRegion, UARTRegion,
};
use crate::simulator::{DynDevice, Simulator, SIMULATOR};
use std::cell::RefCell;
use std::process::Stdio;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

mod protocol;
mod regions;
mod simulator;
mod vector_table;

use crate::protocol::{Request, Response, CMD_READ, CMD_WRITE, RESP_DATA, RESP_ERROR};

async fn init_sim() {
    println!("Parent: Connecting to DevConsole...");
    let dev = DynDevice::new("ws://localhost:9001").await;
    println!("Parent: Connected to DevConsole.");

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
    handlers.push(BXCanRegion::new_boxed(dev.clone(), 0x40006400).await);
    handlers.push(SCBRegion::new_boxed(dev.clone(), 0xE000_ED00));
    handlers.push(NVICRegion::new_boxed(dev.clone(), 0xE000_E100));
    handlers.push(BasicTimerRegion::new_boxed(dev.clone(), 0x40001000));

    let sim = Simulator::new(dev, handlers);
    let sim = RefCell::new(sim);

    match SIMULATOR.set(Mutex::new(sim)) {
        Ok(_) => (),
        Err(_) => panic!("Simulator is already initialized"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <app_executable> [args...]", args[0]);
        std::process::exit(1);
    }

    let app_path = &args[1];
    let app_args = &args[2..];

    println!("Initializing MCU Simulator...");
    init_sim().await;

    println!("Parent: Spawning App Process: {} {:?}", app_path, app_args);
    let mut child = Command::new(app_path)
        .args(app_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut child_stdin = child.stdin.take().unwrap();
    let mut child_stdout = child.stdout.take().unwrap();

    let mut request_buf = [0u8; std::mem::size_of::<Request>()];

    println!("Parent: Entering main loop...");
    loop {
        match child_stdout.read_exact(&mut request_buf).await {
            Ok(_) => {
                let req: Request =
                    unsafe { std::ptr::read_unaligned(request_buf.as_ptr() as *const Request) };
                let req_addr = req.address;
                let req_cmd = req.cmd;
                let req_val = req.value;

                let mut response = Response {
                    resp_type: RESP_DATA,
                    _reserved: [0; 7],
                    value: 0,
                };

                let sim_mutex = SIMULATOR.get().unwrap();

                if let Some(handler) = sim_mutex
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .lookup_handler(req_addr as usize)
                {
                    match req_cmd {
                        CMD_READ => {
                            let val = handler.read(req_addr as usize) as u64;
                            response.value = val;
                            println!("Parent: Read from 0x{:08X} ==> 0x{:08X}", req_addr, val);
                        }
                        CMD_WRITE => {
                            println!("Parent: Write to 0x{:08X} <== 0x{:08X}", req_addr, req_val);
                            handler.write(req_addr as usize, req_val);
                        }
                        _ => {
                            response.resp_type = RESP_ERROR;
                        }
                    }
                } else {
                    response.resp_type = RESP_ERROR;
                }

                let resp_bytes: [u8; 16] = unsafe { std::mem::transmute(response) };
                child_stdin.write_all(&resp_bytes).await?;
                child_stdin.flush().await?;
            }
            Err(e) => {
                eprintln!("Parent: Error reading from child: {}", e);
                break;
            }
        }
    }

    let status = child.wait().await?;
    println!("App Process exited with status: {}", status);

    Ok(())
}
