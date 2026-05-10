use crate::regions::bxcan_region::can_message::CANMessage;

use super::mailbox::Mailbox;

const MAILBOX_COUNT: usize = 3;

pub struct TXMailbox {
    mailboxes: [Mailbox; MAILBOX_COUNT],
    last_send_mailbox: usize,
}

impl TXMailbox {
    pub fn new() -> Self {
        Self {
            mailboxes: [Mailbox::new(), Mailbox::new(), Mailbox::new()],
            last_send_mailbox: 0,
        }
    }

    pub fn encode_tsr(&self) -> u32 {
        let mut tsr = 0u32;
        let lowest_priority_mailbox = self.last_send_mailbox;

        let next_free_mailbox = ((1 + self.last_send_mailbox) % MAILBOX_COUNT) as u32;

        tsr |= 0x20 << lowest_priority_mailbox;
        tsr |= next_free_mailbox << 24;

        for i in 0..3 {
            tsr |= (self.mailboxes[i].encode_tsr() as u32) << (i * 8);
            tsr |= if self.mailboxes[i].is_empty() {
                1 << (26 + i)
            } else {
                0
            };
        }
        return tsr;
    }
    pub fn read(&self, offset: usize) -> Option<u32> {
        if offset == 0x008 {
            return Some(self.encode_tsr());
        }

        if offset == 0x180 {
            return Some(self.mailboxes[0].encode_tir());
        }
        if offset == 0x184 {
            return Some(self.mailboxes[0].encode_tdtr());
        }
        if offset == 0x188 {
            return Some(self.mailboxes[0].encode_tdlr());
        }
        if offset == 0x18C {
            return Some(self.mailboxes[0].encode_tdhr());
        }
        if offset == 0x190 {
            return Some(self.mailboxes[1].encode_tir());
        }
        if offset == 0x194 {
            return Some(self.mailboxes[1].encode_tdtr());
        }
        if offset == 0x198 {
            return Some(self.mailboxes[1].encode_tdlr());
        }
        if offset == 0x19C {
            return Some(self.mailboxes[1].encode_tdhr());
        }
        if offset == 0x1A0 {
            return Some(self.mailboxes[2].encode_tir());
        }
        if offset == 0x1A4 {
            return Some(self.mailboxes[2].encode_tdtr());
        }
        if offset == 0x1A8 {
            return Some(self.mailboxes[2].encode_tdlr());
        }
        if offset == 0x1AC {
            return Some(self.mailboxes[2].encode_tdhr());
        }

        None
    }

    pub fn write(&mut self, offset: usize, value: u32) -> Option<CANMessage> {
        if offset == 0x008 {
            let abrq1 = (value & 0x00800000) != 0;
            let abrq2 = (value & 0x00008000) != 0;
            let abrq3 = (value & 0x00000080) != 0;
            if abrq1 {
                self.mailboxes[0].abort();
            }
            if abrq2 {
                self.mailboxes[1].abort();
            }
            if abrq3 {
                self.mailboxes[2].abort();
            }
        }
        if offset == 0x180 {
            self.mailboxes[0].write_tir(value);
            if value & 1 != 0 {
                return Some(self.mailboxes[0].into());
            }
        }
        if offset == 0x184 {
            self.mailboxes[0].write_tdtr(value);
        }
        if offset == 0x188 {
            self.mailboxes[0].write_tdlr(value);
        }
        if offset == 0x18C {
            self.mailboxes[0].write_tdhr(value);
        }

        if offset == 0x190 {
            self.mailboxes[1].write_tir(value);
            if value & 1 != 0 {
                return Some(self.mailboxes[1].into());
            }
        }
        if offset == 0x194 {
            self.mailboxes[1].write_tdtr(value);
        }
        if offset == 0x198 {
            self.mailboxes[1].write_tdlr(value);
        }
        if offset == 0x19C {
            self.mailboxes[1].write_tdhr(value);
        }

        if offset == 0x1A0 {
            self.mailboxes[2].write_tir(value);
            if value & 1 != 0 {
                return Some(self.mailboxes[2].into());
            }
        }
        if offset == 0x1A4 {
            self.mailboxes[2].write_tdtr(value);
        }
        if offset == 0x1A8 {
            self.mailboxes[2].write_tdlr(value);
        }
        if offset == 0x1AC {
            self.mailboxes[2].write_tdhr(value);
        }

        return None;
    }
}
