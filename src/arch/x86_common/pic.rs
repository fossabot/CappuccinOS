// Originally from pic8259 (https://docs.rs/pic8259/0.10.1/pic8259/)
// But this one feeds my addiction of not adding unnecessary crates
// And I can read and learn about the PIC too ig
// Driver for the 8086 PIC, we might switch to the APIC later on.

use super::io::{inb, outb};

// Command used to begin PIC initialization.
const CMD_INIT: u8 = 0x11;

// Command used to acknowledge an interrupt.
const CMD_END_OF_INTERRUPT: u8 = 0x20;

// The mode we want to run the PIC in.
const MODE_8086: u8 = 0x01;

struct Pic {
    // The base interrupt offset
    offset: u8,

    // The I/O port we send commands to.
    command: u8,

    // The I/O port we receive commands from
    data: u8,
}

impl Pic {
    // Are we in charge of handling this interrupt?
    fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        return self.offset <= interrupt_id && interrupt_id < self.offset + 8;
    }

    // Notify us that an interrupt has been handled and we're ready
    // for more
    fn end_of_interrupt(&mut self) {
        return outb(self.command as u16, CMD_END_OF_INTERRUPT);
    }

    // Reads the interrupt mask of this PIC.
    fn read_mask(&mut self) -> u8 {
        return inb(self.data as u16);
    }

    // Write the interrupt mask of this PIC
    fn write_mask(&mut self, mask: u8) {
        return outb(self.data as u16, mask);
    }
}

// A pair of chained Pic controllers.
pub struct ChainedPics {
    pics: [Pic; 2],
}

impl ChainedPics {
    // Create a new interface for the chained pic controllers.
    pub const fn new(offset1: u8, offset2: u8) -> ChainedPics {
        ChainedPics {
            pics: [
                Pic {
                    offset: offset1,
                    command: 0x20,
                    data: 0x21,
                },
                Pic {
                    offset: offset2,
                    command: 0xA0,
                    data: 0xA1,
                },
            ],
        }
    }

    // Initialize the chained pic controllers.
    pub fn initialize(&mut self) {
        // We need to delay writes to our PICs incase we are on a slower
        // machine. writing to port 0x80 should take care of this.
        let wait_port: u8 = 0x80;
        let wait = || outb(wait_port as u16, 0);

        // Tell each PIC we're going to initialize it.
        outb(self.pics[0].command as u16, CMD_INIT);
        wait();
        outb(self.pics[1].command as u16, CMD_INIT);
        wait();

        // Byte 1: Set up our base offsets.
        outb(self.pics[0].data as u16, self.pics[0].offset);
        wait();
        outb(self.pics[1].data as u16, self.pics[1].offset);
        wait();

        // Byte 2: Configure chaining
        outb(self.pics[0].data as u16, 4); // Tell Maste Pic that there is a slave Pic at IRQ2
        wait();
        outb(self.pics[1].data as u16, 2); // Tell Slave PIC it's cascade identity
        wait();

        // Byte 3: Set out mode.
        outb(self.pics[0].data as u16, MODE_8086);
        wait();
        outb(self.pics[1].data as u16, MODE_8086);
        wait();

        crate::libs::logging::log_ok("PICs initialized");
    }

    pub fn read_masks(&mut self) -> [u8; 2] {
        [self.pics[0].read_mask(), self.pics[1].read_mask()]
    }

    pub fn write_masks(&mut self, mask1: u8, mask2: u8) {
        self.pics[0].write_mask(mask1);
        self.pics[1].write_mask(mask2);
    }

    pub fn disable(&mut self) {
        self.write_masks(0xFF, 0xFF);
    }

    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        return self
            .pics
            .iter()
            .any(|pic| pic.handles_interrupt(interrupt_id));
    }

    pub fn notify_end_of_interrupt(&mut self, interrupt_id: u8) {
        if self.handles_interrupt(interrupt_id) {
            if self.pics[1].handles_interrupt(interrupt_id) {
                self.pics[1].end_of_interrupt();
            }
        }
        self.pics[0].end_of_interrupt();
    }
}
