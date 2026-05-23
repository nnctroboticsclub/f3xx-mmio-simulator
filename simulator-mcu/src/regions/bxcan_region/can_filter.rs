use super::fifo_index::FIFOIndex;
use super::filter_scale::FilterScale;

enum CANFilterState {
    Init,
    Active,
}

#[derive(Clone, Copy)]
enum CANFilterType {
    Mask,
    List,
}

#[derive(Clone, Copy)]
struct CANFilter {
    activated: bool,
    fifo_assignment: FIFOIndex,
    scale: FilterScale,
    bank0: u32,
    bank1: u32,
    filter_type: CANFilterType,
}

impl CANFilter {
    pub fn new() -> Self {
        Self {
            activated: false,
            fifo_assignment: FIFOIndex::FIFO0,
            scale: FilterScale::Scale16Bit,
            bank0: 0,
            bank1: 0,
            filter_type: CANFilterType::Mask,
        }
    }
}

pub struct CANFilters {
    filters: [CANFilter; 28],
    state: CANFilterState,
}

impl CANFilters {
    pub fn new() -> Self {
        Self {
            filters: [CANFilter::new(); 28],
            state: CANFilterState::Init,
        }
    }

    fn encode_master_register(&self) -> u32 {
        0x2A1C0E00
            | match self.state {
                CANFilterState::Init => 1,
                CANFilterState::Active => 0,
            }
    }

    fn write_master_register(&mut self, value: u32) {
        if value & 0xFFFFFFFE != 0x2A1C0E00 {
            panic!(
                "Write to bxCAN filter register with unsupported value {:08x}",
                value
            );
        }
        if value & 0x1 != 0 {
            self.state = CANFilterState::Active;
        } else {
            self.state = CANFilterState::Init;
        }
    }

    fn encode_mode_register(&self) -> u32 {
        let mut value = 0;
        for i in 0..13 {
            if let CANFilterType::List = self.filters[i].filter_type {
                value |= 1 << i;
            }
        }
        value
    }

    fn write_mode_register(&mut self, value: u32) {
        for i in 0..13 {
            let filter_id = i;
            self.filters[i].filter_type = if value & (1 << filter_id) != 0 {
                CANFilterType::List
            } else {
                CANFilterType::Mask
            };
        }
    }

    fn encode_scale_register(&self) -> u32 {
        let mut value = 0;
        for i in 0..13 {
            if let FilterScale::Scale32Bit = self.filters[i].scale {
                value |= 1 << i;
            }
        }
        value
    }

    fn write_scale_register(&mut self, value: u32) {
        if value & 0xFFFFFC00 != 0 {
            panic!(
                "Write to bxCAN filter scale register with unsupported value {:08x}",
                value
            );
        }
        for i in 0..13 {
            let filter_id = i;
            self.filters[i].scale = if value & (1 << filter_id) != 0 {
                FilterScale::Scale32Bit
            } else {
                FilterScale::Scale16Bit
            };
        }
    }

    fn encode_filter_assignment_register(&self) -> u32 {
        let mut value = 0;
        for i in 0..13 {
            if let FIFOIndex::FIFO1 = self.filters[i].fifo_assignment {
                value |= 1 << i;
            }
        }
        value
    }

    fn write_filter_assignment_register(&mut self, value: u32) {
        if value & 0xFFFFFE00 != 0 {
            panic!(
                "Write to bxCAN filter FIFO assignment register with unsupported value {:08x}",
                value
            );
        }
        for i in 0..13 {
            let filter_id = i;
            self.filters[i].fifo_assignment = if value & (1 << filter_id) != 0 {
                FIFOIndex::FIFO1
            } else {
                FIFOIndex::FIFO0
            };
        }
    }

    fn encode_activation_register(&self) -> u32 {
        let mut value = 0;
        for i in 0..13 {
            if self.filters[i].activated {
                value |= 1 << i;
            }
        }
        value
    }

    fn write_activation_register(&mut self, value: u32) {
        if value & 0xFFFFC000 != 0 {
            panic!(
                "Write to bxCAN filter activation register with unsupported value {:08x}",
                value
            );
        }
        for i in 0..13 {
            let filter_id = i;
            let activated = (value & (1 << filter_id)) != 0;
            self.filters[i].activated = activated;
        }
    }

    fn encode_bank_register(&self, offset: usize) -> u32 {
        let sub_offset = offset - 0x40;
        let filter_id = sub_offset / 2;
        let bank_part = sub_offset % 2;
        if filter_id >= 13 {
            panic!(
                "Read from undefined bxCAN filter register at +{:08x}",
                offset
            );
        }

        let filter = self.filters[filter_id];
        match bank_part {
            0 => filter.bank0,
            1 => filter.bank1,
            _ => unreachable!(),
        }
    }

    fn write_bank_register(&mut self, offset: usize, value: u32) {
        let sub_offset = offset - 0x40;
        let filter_id = sub_offset / 2;
        let bank_part = sub_offset % 2;
        if filter_id >= 13 {
            panic!(
                "Write to undefined bxCAN filter register at +{:08x} with value {:08x}",
                offset, value
            );
        }

        let filter = &mut self.filters[filter_id];
        match bank_part {
            0 => filter.bank0 = value,
            1 => filter.bank1 = value,
            _ => unreachable!(),
        };
    }

    pub fn handle_write(&mut self, offset: usize, value: u32) -> bool {
        if offset == 0x00 {
            self.write_master_register(value);
            return true;
        }

        if offset == 0x04 {
            self.write_mode_register(value);
            return true;
        }

        if offset == 0x0C {
            self.write_scale_register(value);
            return true;
        }

        if offset == 0x14 {
            self.write_filter_assignment_register(value);
            return true;
        }

        if offset == 0x1C {
            self.write_activation_register(value);
            return true;
        }

        if (0x40..=0xAC).contains(&offset) {
            self.write_bank_register(offset, value);
            return true;
        }

        false
    }

    pub fn handle_read(&self, offset: usize) -> Option<u32> {
        if offset == 0x00 {
            return Some(self.encode_master_register());
        }
        if offset == 0x04 {
            return Some(self.encode_mode_register());
        }
        if offset == 0x0C {
            return Some(self.encode_scale_register());
        }
        if offset == 0x14 {
            return Some(self.encode_filter_assignment_register());
        }
        if offset == 0x1C {
            return Some(self.encode_activation_register());
        }

        if (0x40..=0xAC).contains(&offset) {
            return Some(self.encode_bank_register(offset));
        }

        None
    }

    pub fn route_message(&self, id: u32, is_extended_id: bool) -> Option<FIFOIndex> {
        for filter in self.filters.iter() {
            if !filter.activated {
                continue;
            }
            let id_masked = if is_extended_id {
                id & 0x1FFFFFFF
            } else {
                id & 0x7FF
            };
            let filter_id_masked = if is_extended_id {
                filter.bank0 & 0x1FFFFFFF
            } else {
                filter.bank0 & 0x7FF
            };
            let mask = if is_extended_id {
                if let CANFilterType::Mask = filter.filter_type {
                    filter.bank1 & 0x1FFFFFFF
                } else {
                    0x1FFFFFFF
                }
            } else if let CANFilterType::Mask = filter.filter_type {
                filter.bank1 & 0x7FF
            } else {
                0x7FF
            };
            if (id_masked ^ filter_id_masked) & mask == 0 {
                return Some(filter.fifo_assignment);
            }
        }
        None
    }
}
