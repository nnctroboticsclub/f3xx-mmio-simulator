use crate::simulator::DynDevice;

use super::MmioHandler;

const GPIO_REGION_SIZE: usize = 0x2C;

pub enum GPIOPort {
    A,
    B,
    C,
    D,
    F,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum GPIOMode {
    Input,
    Output,
    AlternateFunction,
    Analog,
}

#[derive(Clone, Copy, Debug)]
enum GPIOOutputSpeed {
    Low0,
    Low1,
    Medium,
    High,
}

impl From<u32> for GPIOOutputSpeed {
    fn from(value: u32) -> Self {
        match value {
            0b00 => GPIOOutputSpeed::Low0,
            0b01 => GPIOOutputSpeed::Low1,
            0b10 => GPIOOutputSpeed::Medium,
            0b11 => GPIOOutputSpeed::High,
            _ => unreachable!(),
        }
    }
}

impl From<GPIOOutputSpeed> for u32 {
    fn from(val: GPIOOutputSpeed) -> Self {
        match val {
            GPIOOutputSpeed::Low0 => 0b00,
            GPIOOutputSpeed::Low1 => 0b01,
            GPIOOutputSpeed::Medium => 0b10,
            GPIOOutputSpeed::High => 0b11,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum GPIOPull {
    None,
    PullUp,
    PullDown,
    Reserved,
}

impl From<u32> for GPIOPull {
    fn from(value: u32) -> Self {
        match value {
            0b00 => GPIOPull::None,
            0b01 => GPIOPull::PullUp,
            0b10 => GPIOPull::PullDown,
            0b11 => GPIOPull::Reserved,
            _ => unreachable!(),
        }
    }
}

impl From<GPIOPull> for u32 {
    fn from(val: GPIOPull) -> Self {
        match val {
            GPIOPull::None => 0b00,
            GPIOPull::PullUp => 0b01,
            GPIOPull::PullDown => 0b10,
            GPIOPull::Reserved => 0b11,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GPIOPin {
    mode: GPIOMode,
    output_open_drain: bool,
    output_speed: GPIOOutputSpeed,
    pull: GPIOPull,
    alternate_function: u8,
    output: bool,
    input: bool,
}
impl GPIOPin {
    fn new() -> Self {
        Self {
            mode: GPIOMode::Input,
            output_open_drain: false,
            output_speed: GPIOOutputSpeed::Low0,
            pull: GPIOPull::None,
            alternate_function: 0,
            output: false,
            input: false,
        }
    }

    fn read_input(&self) -> bool {
        self.input
    }

    fn write_output(&mut self, value: bool) {
        if self.output == value {
            return;
        }
        if self.mode == GPIOMode::Output {
            self.output = value;
            self.input = value;
        } else {
            println!("GPIO: {self:#?}");
            panic!("Attempt to write to a GPIO pin that is not in output mode");
        }
    }
}

pub struct GPIORegion {
    start_addr: usize,
    _port: GPIOPort,
    pins: [GPIOPin; 16],
}
impl GPIORegion {
    fn new(_dev: DynDevice, start_addr: usize, port: GPIOPort) -> Self {
        Self {
            start_addr,
            _port: port,
            pins: [GPIOPin::new(); 16],
        }
    }
    pub fn new_boxed(dev: DynDevice, start_addr: usize, port: GPIOPort) -> Box<Self> {
        Box::new(Self::new(dev, start_addr, port))
    }
    pub fn new_gpioa(dev: DynDevice, start_addr: usize) -> Box<Self> {
        let mut region = Self::new(dev, start_addr, GPIOPort::A);
        region.pins[15].mode = GPIOMode::AlternateFunction;
        region.pins[14].mode = GPIOMode::AlternateFunction;
        region.pins[13].mode = GPIOMode::AlternateFunction;
        region.pins[13].output_speed = GPIOOutputSpeed::High;
        region.pins[15].pull = GPIOPull::PullDown;
        region.pins[14].pull = GPIOPull::PullUp;
        region.pins[13].pull = GPIOPull::PullDown;
        Box::new(region)
    }
    pub fn new_gpiob(dev: DynDevice, start_addr: usize) -> Box<Self> {
        let mut region = Self::new(dev, start_addr, GPIOPort::B);
        region.pins[4].mode = GPIOMode::AlternateFunction;
        region.pins[3].mode = GPIOMode::AlternateFunction;
        region.pins[3].output_speed = GPIOOutputSpeed::High;
        region.pins[4].pull = GPIOPull::PullUp;
        Box::new(region)
    }
}

impl MmioHandler for GPIORegion {
    fn read(&self, address: usize) -> u32 {
        let offset = address - self.start_addr;

        if offset == 0 {
            let moder = self
                .pins
                .iter()
                .enumerate()
                .fold(0, |acc, (i, pin)| acc | ((pin.mode as u32) << (i * 2)));
            moder
        } else if offset == 0x04 {
            let otyper = self.pins.iter().enumerate().fold(0, |acc, (i, pin)| {
                acc | ((pin.output_open_drain as u32) << i)
            });
            otyper
        } else if offset == 0x08 {
            let ospeedr = self.pins.iter().enumerate().fold(0, |acc, (i, pin)| {
                acc | ((Into::<u32>::into(pin.output_speed)) << (i * 2))
            });
            ospeedr
        } else if offset == 0x0C {
            let pupdr = self
                .pins
                .iter()
                .enumerate()
                .fold(0, |acc, (i, pin)| acc | ((pin.pull as u32) << (i * 2)));
            pupdr
        } else if offset == 0x10 {
            let idr = self
                .pins
                .iter()
                .enumerate()
                .fold(0, |acc, (i, pin)| acc | ((pin.read_input() as u32) << i));
            idr
        } else if offset == 0x14 {
            let odr = self
                .pins
                .iter()
                .enumerate()
                .fold(0, |acc, (i, pin)| acc | ((pin.output as u32) << i));
            odr
        } else if offset == 0x20 {
            let afrl = self.pins[0..8].iter().enumerate().fold(0, |acc, (i, pin)| {
                acc | ((pin.alternate_function as u32) << (i * 4))
            });
            afrl
        } else if offset == 0x24 {
            let afrh = self.pins[8..16]
                .iter()
                .enumerate()
                .fold(0, |acc, (i, pin)| {
                    acc | ((pin.alternate_function as u32) << (i * 4))
                });
            afrh
        } else {
            panic!("Read from undefined GPIO region at address {:08x}", address);
        }
    }
    fn write(&mut self, address: usize, value: u64) {
        let value = value as u32;
        let offset = address - self.start_addr;

        if offset == 0x00 {
            for (pin, i) in self.pins.iter_mut().zip(0..) {
                let mode_bits = (value >> (i * 2)) & 0b11;
                pin.mode = match mode_bits {
                    0b00 => GPIOMode::Input,
                    0b01 => GPIOMode::Output,
                    0b10 => GPIOMode::AlternateFunction,
                    0b11 => GPIOMode::Analog,
                    _ => unreachable!(),
                };
            }
        } else if offset == 0x04 {
            for (pin, i) in self.pins.iter_mut().zip(0..) {
                let otype_bit = (value >> i) & 0b1;
                pin.output_open_drain = otype_bit != 0;
            }
        } else if offset == 0x08 {
            for (pin, i) in self.pins.iter_mut().zip(0..) {
                let ospeed_bits = (value >> (i * 2)) & 0b11;
                pin.output_speed = GPIOOutputSpeed::from(ospeed_bits);
            }
        } else if offset == 0x0C {
            for (pin, i) in self.pins.iter_mut().zip(0..) {
                let pull_bits = (value >> (i * 2)) & 0b11;
                pin.pull = match pull_bits {
                    0b00 => GPIOPull::None,
                    0b01 => GPIOPull::PullUp,
                    0b10 => GPIOPull::PullDown,
                    _ => unreachable!(),
                };
            }
        } else if offset == 0x14 {
            for (pin, i) in self.pins.iter_mut().zip(0..) {
                let output_bit = (value >> i) & 0b1;
                pin.write_output(output_bit != 0);
            }
        } else if offset == 0x20 {
            for (pin, i) in self.pins[0..8].iter_mut().zip(0..) {
                let af_bits = (value >> (i * 4)) & 0b1111;
                pin.alternate_function = af_bits as u8;
            }
        } else if offset == 0x24 {
            for (pin, i) in self.pins[8..16].iter_mut().zip(0..) {
                let af_bits = (value >> (i * 4)) & 0b1111;
                pin.alternate_function = af_bits as u8;
            }
        } else {
            panic!(
                "Write to undefined GPIO region at address {:08x} with value {:08x}",
                address, value
            );
        }
    }
    fn contains(&self, address: usize) -> bool {
        address >= self.start_addr && address < self.start_addr + GPIO_REGION_SIZE
    }
}
