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

mod regions;
mod simulator;
mod vector_table;

use ipc_protocol::{Request, Response, CMD_READ, CMD_READY, CMD_WRITE, RESP_DATA, RESP_ERROR};

/// --- Hardware Constants (STM32F303x8) ---
pub const FLASH_BASE: usize = 0x4002_2000;
pub const RCC_BASE: usize = 0x4002_1000;
pub const GPIOA_BASE: usize = 0x4800_0000;
pub const GPIOB_BASE: usize = 0x4800_0400;
pub const GPIOC_BASE: usize = 0x4800_0800;
pub const USART2_BASE: usize = 0x4000_4400;
pub const BXCAN_BASE: usize = 0x4000_6400;
pub const TIM6_BASE: usize = 0x4000_1000;
pub const NVIC_BASE: usize = 0xE000_E100;
pub const SCB_BASE: usize = 0xE000_ED00;

async fn init_sim() {
    println!("Parent: Connecting to DevConsole...");
    let dev = DynDevice::new("ws://localhost:9001").await;
    println!("Parent: Connected to DevConsole.");

    let mut handlers = Vec::<DynMMIOHandler>::new();
    handlers.push(FlashRegion::new_boxed(dev.clone(), FLASH_BASE));
    handlers.push(RCCRegion::new_boxed(dev.clone(), RCC_BASE));
    handlers.push(BridgeRegion::new_boxed(dev.clone(), 0xabcd_0000));
    handlers.push(GPIORegion::new_gpioa(dev.clone(), GPIOA_BASE));
    handlers.push(GPIORegion::new_gpiob(dev.clone(), GPIOB_BASE));
    handlers.push(GPIORegion::new_boxed(dev.clone(), GPIOC_BASE, GPIOPort::C));
    handlers.push(UARTRegion::new_boxed(dev.clone(), USART2_BASE, 2));
    handlers.push(BXCanRegion::new_boxed(dev.clone(), BXCAN_BASE).await);
    handlers.push(SCBRegion::new_boxed(dev.clone(), SCB_BASE));
    handlers.push(NVICRegion::new_boxed(dev.clone(), NVIC_BASE));
    handlers.push(BasicTimerRegion::new_boxed(dev.clone(), TIM6_BASE));

    SIMULATOR
        .set(Mutex::new(RefCell::new(Simulator::new(dev, handlers))))
        .ok();
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

    println!("Parent: Waiting for READY packet from child...");
    loop {
        child_stdout.read_exact(&mut request_buf).await?;
        let req: Request =
            unsafe { std::ptr::read_unaligned(request_buf.as_ptr() as *const Request) };
        let req_cmd = req.cmd;
        let req_addr = req.address;
        if req_cmd == CMD_READY {
            println!("Parent: Received READY. Starting main loop.");
            break;
        } else {
            println!(
                "Parent: Ignoring early request cmd={} addr=0x{:x}",
                req_cmd, req_addr
            );
        }
    }

    println!("Parent: Entering main loop...");
    loop {
        match child_stdout.read_exact(&mut request_buf).await {
            Ok(_) => {
                let req: Request =
                    unsafe { std::ptr::read_unaligned(request_buf.as_ptr() as *const Request) };
                let req_addr = req.address;
                let req_cmd = req.cmd;
                let req_val = req.value;
                let req_seq = req.seq;

                let mut response = Response {
                    resp_type: RESP_DATA,
                    seq: req_seq,
                    _reserved: [0; 5],
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
                            // println!("Parent: Read from 0x{:08X} ==> 0x{:08X}", req_addr, val);
                        }
                        CMD_WRITE => {
                            // println!("Parent: Write to 0x{:08X} <== 0x{:08X}", req_addr, req_val);
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
