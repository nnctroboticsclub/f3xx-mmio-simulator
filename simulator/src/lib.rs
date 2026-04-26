use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    ops::DerefMut,
    sync::{Arc, Mutex, OnceLock},
    thread::sleep,
    time::Duration,
};

use iced_x86::{
    Decoder, Formatter, GasFormatter, Instruction, InstructionInfoFactory, Mnemonic, OpKind,
    Register,
};
use nix::{
    libc::{self, mcontext_t},
    sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
};

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        fn init_mmio_simulator();
    }
}

trait MmioHandler {
    fn read(&self, address: usize) -> u32;
    fn write(&mut self, address: usize, value: u32);
    fn contains(&self, address: usize) -> bool;
}
type DynMMIOHandler = Box<dyn MmioHandler + Send + Sync>;
impl Debug for DynMMIOHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynMMIOHandler")
    }
}

#[derive(Debug)]
struct Simulator {
    handlers: Vec<DynMMIOHandler>,
}

impl Simulator {
    fn lookup_handler(&mut self, address: usize) -> Option<&mut DynMMIOHandler> {
        self.handlers
            .iter_mut()
            .find(|handler| handler.contains(address))
    }
}

static SIMULATOR: OnceLock<Mutex<RefCell<Simulator>>> = OnceLock::new();

fn reraise_as_native_segv() {
    unsafe {
        libc::signal(libc::SIGSEGV, libc::SIG_DFL);
        libc::raise(libc::SIGSEGV);
        libc::exit(1);
    }
}

fn iced_register_to_libc_reg(reg: Register) -> Option<libc::c_int> {
    match reg {
        Register::RAX | Register::EAX => Some(libc::REG_RAX),
        Register::RCX | Register::ECX => Some(libc::REG_RCX),
        Register::RDX | Register::EDX => Some(libc::REG_RDX),
        Register::RBX | Register::EBX => Some(libc::REG_RBX),
        Register::RSP | Register::ESP => Some(libc::REG_RSP),
        Register::RBP | Register::EBP => Some(libc::REG_RBP),
        Register::RSI | Register::ESI => Some(libc::REG_RSI),
        Register::RDI | Register::EDI => Some(libc::REG_RDI),
        _ => None,
    }
}

struct Context<'a> {
    mcontext: &'a mut mcontext_t,
    region: &'a mut DynMMIOHandler,
}

impl<'a> Context<'a> {
    fn get_register_value(&self, reg: Register) -> Option<u64> {
        if let Some(libc_reg) = iced_register_to_libc_reg(reg) {
            return Some(self.mcontext.gregs[libc_reg as usize] as u64);
        }
        match reg {
            Register::ES | Register::CS | Register::SS | Register::DS => Some(0),
            _ => {
                panic!("Unsupported register access: {:?}", reg);
            }
        }
    }
    fn write_register(&mut self, reg: Register, value: u64) {
        if let Some(libc_reg) = iced_register_to_libc_reg(reg) {
            self.mcontext.gregs[libc_reg as usize] = value as i64;
        } else {
            panic!("Unsupported register for writing: {:?}", reg);
        }
    }

    fn write_u32_memory(&mut self, address: usize, value: u32) {
        self.region.write(address, value);
    }

    fn read_u32_memory(&self, address: usize) -> u32 {
        self.region.read(address)
    }

    pub fn set_operand_value(&mut self, inst: Instruction, op_index: u32, value: u32) {
        match inst.op_kind(op_index) {
            OpKind::Register => {
                let reg = inst.op_register(op_index);
                self.write_register(reg, value as u64);
            }
            OpKind::Memory => {
                let address = inst
                    .virtual_address(op_index, 0, |reg, _, _| self.get_register_value(reg))
                    .unwrap() as usize;
                self.write_u32_memory(address, value);
            }
            _ => {
                panic!(
                    "Unsupported operand kind for writing: {:?}",
                    inst.op_kind(op_index)
                );
            }
        }
    }

    pub fn get_operand_value(&self, index: u32, inst: &Instruction) -> Option<u64> {
        if let Ok(imm) = inst.try_immediate(index) {
            return Some(imm);
        }

        match inst.op_kind(index) {
            OpKind::Register => {
                let reg = inst.op_register(index);
                self.get_register_value(reg)
            }
            OpKind::Memory => {
                let address = inst
                    .virtual_address(index, 0, |reg, _, _| self.get_register_value(reg))
                    .unwrap() as usize;

                Some(self.read_u32_memory(address) as u64)
            }
            _ => {
                println!(
                    "Unsupported operand kind for operand {index}: {:?}",
                    inst.op_kind(index)
                );
                None
            }
        }
    }
}

fn diagnose_inst(mcontext: &mcontext_t, inst: Instruction) {
    let mut info_factory = InstructionInfoFactory::new();
    let info = info_factory.info(&inst);

    let mut inst_formatter = GasFormatter::new();

    let get_register_value = |reg, _index, _size| match reg {
        Register::ES | Register::CS | Register::SS | Register::DS => Some(0),
        _ => Some(mcontext.gregs[reg as usize] as u64),
    };

    let out = {
        let mut out = String::new();
        inst_formatter.format(&inst, &mut out);
        out
    };
    println!("@PC + 0x00: {:?}: {out}", inst.mnemonic());
    for i in 0..inst.op_count() {
        let kind = inst.op_kind(i);
        let access = info.op_access(i);
        match kind {
            OpKind::Memory => {
                let va = inst.virtual_address(i, 0, get_register_value);
                if va.is_none() {
                    eprintln!("Failed to compute the virtual address");
                    reraise_as_native_segv();
                }
                let va = va.unwrap() as usize;
                println!("- Memory access at address {va:#x} as {access:?}");
            }
            OpKind::Register => {
                let reg = inst.op_register(i);
                if let Some(reg) = iced_register_to_libc_reg(reg) {
                    let value = mcontext.gregs[reg as usize];
                    println!("- Register access: {reg:?} with value {value:#x} as {access:?}");
                } else {
                    println!("- Register access: {reg:?} as {access:?}");
                }
            }
            _ => {
                println!("- Other operand: {kind:?} as {access:?}");
            }
        }
    }
}

extern "C" fn mmio_segv_handler(
    _signum: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let address = unsafe { info.read().si_addr() as usize };
    let ucontext = unsafe { &mut *(context as *mut libc::ucontext_t) };
    let mcontext = &mut ucontext.uc_mcontext;
    let pc = mcontext.gregs[libc::REG_RIP as usize] as *const u8;

    let simulator = SIMULATOR
        .get()
        .expect("Simulator not initialized")
        .lock()
        .expect("Failed to lock the simulator");
    let mut simulator = simulator.borrow_mut();
    let handler = simulator.lookup_handler(address);
    if handler.is_none() {
        eprintln!("Segmentation fault at address {address:#x} (PC: {pc:?}) with no handler found");
        reraise_as_native_segv();
    }
    let handler = handler.unwrap();

    let mut decoder = Decoder::new(
        64,
        unsafe { std::slice::from_raw_parts(pc, 15) },
        iced_x86::DecoderOptions::NONE,
    );
    let inst = decoder.decode();

    let mut ctx = Context {
        mcontext: mcontext,
        region: handler,
    };

    match inst.mnemonic() {
        Mnemonic::Or => {
            let value = ctx.get_operand_value(0, &inst).unwrap() as u32;
            let operand_value = ctx.get_operand_value(1, &inst).unwrap() as u32;
            let new_value = value | operand_value;
            ctx.set_operand_value(inst, 0, new_value);
        }
        Mnemonic::And => {
            let op0 = ctx.get_operand_value(0, &inst).unwrap() as u32;
            let op1 = ctx.get_operand_value(1, &inst).unwrap() as u32;
            let new_value = op0 & op1;
            ctx.set_operand_value(inst, 0, new_value);
        }
        Mnemonic::Mov => {
            let value = ctx.get_operand_value(1, &inst).unwrap() as u32;
            ctx.set_operand_value(inst, 0, value);
        }
        Mnemonic::Test => {
            let value = ctx.get_operand_value(0, &inst).unwrap() as u32;
            let operand_value = ctx.get_operand_value(1, &inst).unwrap() as u32;
            let new_value = value & operand_value;

            let flags = &mut mcontext.gregs[libc::REG_EFL as usize];
            *flags &= !(1 << 0); // CF
            *flags &= !(1 << 6); // ZF
            *flags &= !(1 << 7); // SF
            *flags &= !(1 << 2); // PF
            *flags &= !!(1 << 11); // OF
            if (new_value & 0x80) != 0 {
                *flags |= 1 << 7; // SF
            }
            if new_value == 0 {
                *flags |= 1 << 6; // ZF
            }
            let mut parity = 0;
            for i in 0..8 {
                if (new_value & (1 << i)) != 0 {
                    parity += 1;
                }
            }
            if parity % 2 == 0 {
                *flags |= 1 << 2; // PF
            }
        }
        _ => {
            diagnose_inst(mcontext, inst);
            reraise_as_native_segv();
        }
    }

    mcontext.gregs[libc::REG_RIP as usize] += inst.len() as i64;
}

struct MemHandler {
    mem: Vec<u8>,
    start_addr: usize,
    end_addr: usize,
}
impl MemHandler {
    fn new(size: usize, start_addr: usize) -> Self {
        let end_addr = start_addr + size;
        Self {
            mem: vec![0; size],
            start_addr,
            end_addr,
        }
    }
    fn new_boxed(size: usize, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(size, start_addr))
    }
}
impl MmioHandler for MemHandler {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let mut value = 0u32;
        for i in 0..4 {
            value |= (self.mem[offset + i] as u32) << (i * 8);
        }
        println!("R {address:08x} --> {value:08x}");
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;
        println!("W {address:08x} <-- {value:08x}");
        for i in 0..4 {
            self.mem[offset + i] = ((value >> (i * 8)) & 0xFF) as u8;
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.end_addr
    }
}

struct NotImplementedHandler {
    start_addr: usize,
    end_addr: usize,
}
impl NotImplementedHandler {
    fn new(size: usize, start_addr: usize) -> Self {
        let end_addr = start_addr + size;
        Self {
            start_addr,
            end_addr,
        }
    }
    fn new_boxed(size: usize, start_addr: usize) -> Box<Self> {
        Box::new(Self::new(size, start_addr))
    }
}
impl MmioHandler for NotImplementedHandler {
    fn read(&self, address: usize) -> u32 {
        panic!("Read from unimplemented MMIO address {:08x}", address);
    }
    fn write(&mut self, address: usize, value: u32) {
        panic!(
            "Write to unimplemented MMIO address {:08x} with value {:08x}",
            address, value
        );
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.end_addr
    }
}

const FLASH_REGION_SIZE: usize = 0x24;
struct FlashRegion {
    start_addr: usize,
    mem: [u8; FLASH_REGION_SIZE],
}
impl FlashRegion {
    fn new(start_addr: usize) -> Self {
        let end_addr = start_addr + FLASH_REGION_SIZE;
        let mut mem = [0; FLASH_REGION_SIZE];
        mem[0] = 0x03;
        mem[0x20] = 0xFF;
        mem[0x21] = 0xFF;
        mem[0x22] = 0xFF;
        mem[0x23] = 0xFF;
        Self { start_addr, mem }
    }
    fn new_boxed(start_addr: usize) -> Box<Self> {
        Box::new(Self::new(start_addr))
    }
}
impl MmioHandler for FlashRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let defined_behavior = match offset {
            0x00..=0x03 => true,
            _ => false,
        };
        if !defined_behavior {
            panic!(
                "Read from undefined flash region at address {:08x}",
                address
            );
        }
        let mut value = 0u32;
        for i in 0..4 {
            value |= (self.mem[offset + i] as u32) << (i * 8);
        }
        println!("R {address:08x} --> {value:08x}");
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        let offset = address - self.start_addr;
        println!("W {address:08x} <-- {value:08x}");
        if offset == 0x00 {
            // ACR
            self.mem[offset] = (value & 0xFF) as u8;
            self.mem[offset + 1] = ((value >> 8) & 0xFF) as u8;
            self.mem[offset + 2] = ((value >> 16) & 0xFF) as u8;
            self.mem[offset + 3] = ((value >> 24) & 0xFF) as u8;
        } else {
            panic!(
                "Write to undefined flash region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + FLASH_REGION_SIZE
    }
}

const RCC_REGION_SIZE: usize = 0x24;
const RCC_HSI_CLOCK: u32 = 8_000_000;
const RCC_HSE_CLOCK: u32 = 8_000_000;
enum PLLSource {
    HSIDiv2,
    HSE(u32), // prescaler
}
enum SysClockSource {
    HSI,
    HSE,
    PLL,
}
struct RCCRegion {
    start_addr: usize,
    mem: [u32; RCC_REGION_SIZE / 4],
    pll_source: PLLSource,
    pll_multiplier: u32,
    ahb_prescaler: u32,
    apb1_prescaler: u32,
    apb2_prescaler: u32,
    sysclk_source: SysClockSource,
}
impl RCCRegion {
    fn new(start_addr: usize) -> Self {
        let end_addr = start_addr + RCC_REGION_SIZE;
        let mut mem = [0u32; RCC_REGION_SIZE / 4];
        mem[0] = 0x03300083; // PLL, HSE, HSI ON
        mem[5] = 0x00000014;
        mem[8] = 0x00000018;
        Self {
            start_addr,
            mem,
            pll_source: PLLSource::HSIDiv2,
            pll_multiplier: 2,
            ahb_prescaler: 1,
            apb1_prescaler: 1,
            apb2_prescaler: 1,
            sysclk_source: SysClockSource::HSI,
        }
    }
    fn new_boxed(start_addr: usize) -> Box<Self> {
        Box::new(Self::new(start_addr))
    }

    fn set_pll_multiplier_value(&mut self, value: u32) {
        if value == 0x0f {
            self.pll_multiplier = 16;
        } else {
            self.pll_multiplier = value + 2;
        }
    }

    fn get_pll_multiplier_value(&self) -> u32 {
        if self.pll_multiplier == 16 {
            0x0f
        } else {
            self.pll_multiplier - 2
        }
    }

    fn get_pll_hse_prediv_value(&self) -> Option<u32> {
        match self.pll_source {
            PLLSource::HSIDiv2 => None,
            PLLSource::HSE(n) => Some(n - 1),
        }
    }

    fn set_pll_hse_prediv_value(&mut self, value: u32) {
        match self.pll_source {
            PLLSource::HSIDiv2 => {
                panic!("Cannot set HSE predivider when PLL source is HSI/2");
            }
            PLLSource::HSE(_) => {
                self.pll_source = PLLSource::HSE(value + 1);
            }
        }
    }

    fn get_pll_source(&self) -> u32 {
        match self.pll_source {
            PLLSource::HSIDiv2 => 0,
            PLLSource::HSE(_) => 1,
        }
    }

    fn set_pll_source(&mut self, value: u32) {
        match value {
            0 => self.pll_source = PLLSource::HSIDiv2,
            1 => self.pll_source = PLLSource::HSE(1), // default to HSE with no predivision
            _ => panic!("Invalid PLL source value: {value}"),
        }
    }

    fn get_apb1_prescaler_value(&self) -> u32 {
        match self.apb1_prescaler {
            1 => 0,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid APB1 prescaler value: {}", self.apb1_prescaler),
        }
    }
    fn set_apb1_prescaler_value(&mut self, value: u32) {
        self.apb1_prescaler = match value {
            0..=3 => 1,
            4 => 2,
            5 => 4,
            6 => 8,
            7 => 16,
            _ => panic!("Invalid APB1 prescaler setting: {value}"),
        }
    }

    fn get_apb2_prescaler_value(&self) -> u32 {
        match self.apb2_prescaler {
            1 => 0,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid APB2 prescaler value: {}", self.apb2_prescaler),
        }
    }
    fn set_apb2_prescaler_value(&mut self, value: u32) {
        self.apb2_prescaler = match value {
            0..=3 => 1,
            4 => 2,
            5 => 4,
            6 => 8,
            7 => 16,
            _ => panic!("Invalid APB2 prescaler setting: {value}"),
        }
    }
    fn get_ahb_prescaler_value(&self) -> u32 {
        match self.ahb_prescaler {
            1 => 0,
            2 => 8,
            4 => 9,
            8 => 10,
            16 => 11,
            64 => 12,
            128 => 13,
            256 => 14,
            512 => 15,
            _ => panic!("Invalid AHB prescaler value: {}", self.ahb_prescaler),
        }
    }
    fn set_ahb_prescaler_value(&mut self, value: u32) {
        self.ahb_prescaler = match value {
            0..=7 => 1,
            8 => 2,
            9 => 4,
            10 => 8,
            11 => 16,
            12 => 64,
            13 => 128,
            14 => 256,
            15 => 512,
            _ => panic!("Invalid AHB prescaler setting: {value}"),
        }
    }
    fn get_sysclk_source(&self) -> u32 {
        match self.sysclk_source {
            SysClockSource::HSI => 0,
            SysClockSource::HSE => 1,
            SysClockSource::PLL => 2,
        }
    }
    fn set_sysclk_source(&mut self, value: u32) {
        self.sysclk_source = match value {
            0 => SysClockSource::HSI,
            1 => SysClockSource::HSE,
            2 => SysClockSource::PLL,
            _ => panic!("Invalid system clock source value: {value}"),
        }
    }
}
impl MmioHandler for RCCRegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;
        let value = match offset {
            0x00 => self.mem[0], // CR
            0x04 => {
                let mut reg = 0;
                reg |= self.get_pll_multiplier_value() << 18;
                reg |= (self.get_pll_hse_prediv_value().unwrap_or(0) & 1) << 17;
                reg |= self.get_pll_source() << 15;
                reg |= self.get_apb1_prescaler_value() << 11;
                reg |= self.get_apb2_prescaler_value() << 8;
                reg |= self.get_ahb_prescaler_value() << 4;
                reg |= self.get_sysclk_source() << 2;
                reg |= self.get_sysclk_source();
                reg
            } // CFGR
            _ => {
                panic!("Read from undefined RCC region at address {:08x}", address);
            }
        };
        println!("R {address:08x} --> {value:08x}");
        value
    }
    fn write(&mut self, address: usize, value: u32) {
        {
            let start_addr = self.start_addr;
            println!(
                "Attempting to write {value:08x} to RCC@{start_addr:08x} at address {address:08x}"
            );
        }
        let offset = address - self.start_addr;

        let original_value = self.mem[offset / 4];
        let new_value = value;

        let mut changed_bits = original_value ^ new_value;

        println!("W {address:08x} <-- {value:08x}");

        if offset == 0x00 {
            changed_bits &= 0x02040003; // CR ignores PLLON/HSRON/CSSON

            if changed_bits != 0 {
                panic!(
                    "Unhandled RCC->CR write: {:08x} ==> {:08x}",
                    original_value, new_value
                );
            }
        } else if offset == 0x04 {
            changed_bits &= 0x00c3fff3;
            self.mem[offset / 4] = new_value;
            if changed_bits & 0x003c0000 != 0 {
                self.set_pll_multiplier_value((new_value >> 18) & 0xf);
                changed_bits &= !0x003c0000;
            }
            if changed_bits & 0x00000700 != 0 {
                let ppre1 = (new_value >> 8) & 0x7;
                self.set_apb1_prescaler_value(ppre1);
                changed_bits &= !0x00000700;
            }
            if changed_bits & 0x00003800 != 0 {
                let ppre2 = (new_value >> 11) & 0x7;
                self.set_apb2_prescaler_value(ppre2);
                changed_bits &= !0x00003800;
            }
            if changed_bits & 0x000000f0 != 0 {
                let hpre = (new_value >> 4) & 0xf;
                self.set_ahb_prescaler_value(hpre);
                changed_bits &= !0x000000f0;
            }
            if changed_bits & 0x00000003 != 0 {
                self.set_sysclk_source(new_value & 0x3);
                changed_bits &= !0x00000003;
            }
            if changed_bits != 0 {
                panic!(
                    "Unhandled RCC->CFGR write: {:08x} ==> {:08x}",
                    original_value, new_value
                );
            }
        } else {
            panic!(
                "Write to undefined RCC region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + RCC_REGION_SIZE
    }
}

pub extern "C" fn init_mmio_simulator() {
    SIMULATOR
        .set(Mutex::new({
            let mut handlers = Vec::<DynMMIOHandler>::new();
            handlers.push(FlashRegion::new_boxed(0x4002_2000));
            handlers.push(RCCRegion::new_boxed(0x4002_1000));

            let mut sim = Simulator { handlers };
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
