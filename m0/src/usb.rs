//! Polled full-speed USB transport for bounded NOR and SD transactions.
//!
//! Endpoint zero only; the NOR target requires D0 held. M0 writes require opt-in.

use crate::{
    board, dfu, flash,
    regs::*,
    sd,
    storage::{Storage, Target},
};

const BASE: usize = 0x2007_2000;
const CONTROL: usize = BASE + 0x100;
const ADDRESS: usize = BASE + 0x104;
const SOF_TIMER: usize = BASE + 0x110;
const PHY_TEST: usize = BASE + 0x114;
const CX_FIFO: usize = BASE + 0x120;
const MASK_GROUP: usize = BASE + 0x130;
const MASK0: usize = BASE + 0x134;
const MASK1: usize = BASE + 0x138;
const MASK2: usize = BASE + 0x13c;
const EVENTS0: usize = BASE + 0x144;
const EVENTS2: usize = BASE + 0x14c;
const DMA_FIFO: usize = BASE + 0x1c0;
const SETUP_DATA: usize = BASE + 0x1d0;
const DMA_CHANNEL: usize = BASE + 0x300;
const DMA_ADDRESS: usize = BASE + 0x304;
const DMA_EVENTS: usize = BASE + 0x328;
const DMA_ENABLE: usize = BASE + 0x330;
const CLOCK_GATE: usize = 0x2000_0584;
const USB_PLL: usize = 0x2000_0838;
const PHY_CONTROL: usize = 0x2000_e504;
const PDS_CONTROL: usize = 0x2000_e500;

const SOFT_RESET: u32 = 1 << 4;
const CHIP_ENABLE: u32 = 1 << 5;
const FULL_SPEED: u32 = 1 << 9;
const CX_DONE: u32 = 1;
const CX_STALL: u32 = 1 << 2;
const CX_CLEAR: u32 = 1 << 3;
const CX_SETUP: u32 = 1;
const CX_OUT: u32 = 1 << 2;
const CX_ABORT: u32 = 1 << 5;
const CX_FAIL: u32 = 1 << 4;
const BUS_RESET: u32 = 1;
const DMA_COMPLETE: u32 = 1;
const DMA_ERROR: u32 = 1 << 16;
const POLL_LIMIT: usize = 65536;

// pid.codes 1209:0001 is shared, private-testing-only, and not universally unique.
// Do not use this identity for distributed devices: https://pid.codes/1209/0001/
const DEVICE: &[u8] = &[
    18, 1, 0, 2, 0, 0, 0, 64, 0x09, 0x12, 0x01, 0x00, 1, 0, 0, 1, 0, 1,
];
const CONFIG: &[u8] = &[
    9, 2, 36, 0, 1, 1, 0, 0x80, 50, 9, 4, 0, 0, 0, 0xfe, 1, 2, 2, // alternate 0: NOR Flash
    9, 0x21, 7, 0, 0, 64, 0, 0x10, 1, 9, 4, 0, 1, 0, 0xfe, 1, 2, 3, // alternate 1: SD Card
];
const LANG: &[u8] = &[4, 3, 9, 4];
const PRODUCT: &[u8] = &[
    26, 3, b'B', 0, b'L', 0, b'8', 0, b'0', 0, b'8', 0, b' ', 0, b'M', 0, b'0', 0, b' ', 0, b'D',
    0, b'F', 0, b'U', 0,
];
const fn string<const N: usize>(ascii: &[u8]) -> [u8; N] {
    assert!(N == 2 + ascii.len() * 2);
    let mut data = [0; N];
    data[0] = N as u8;
    data[1] = 3;
    let mut i = 0;
    while i < ascii.len() {
        data[2 + i * 2] = ascii[i];
        i += 1;
    }
    data
}

const NOR_TARGET: &[u8] = &string::<20>(b"NOR Flash");
const SD_TARGET: &[u8] = &string::<16>(b"SD Card");

// Readable through the uncached SRAM alias when enumeration needs diagnosis.
// One polling context owns these words; there are no interrupt handlers.
#[unsafe(no_mangle)]
static mut USB_DIAG: [u32; 8] = [0; 8];

fn diag(index: usize, value: u32) {
    unsafe {
        core::ptr::addr_of_mut!(USB_DIAG)
            .cast::<u32>()
            .add(index)
            .write_volatile(value);
    }
}

#[derive(Clone, Copy)]
struct Setup {
    kind: u8,
    request: u8,
    value: u16,
    index: u16,
    length: usize,
}

impl Setup {
    fn decode(first: u32, second: u32) -> Self {
        Self {
            kind: first as u8,
            request: (first >> 8) as u8,
            value: (first >> 16) as u16,
            index: second as u16,
            length: (second >> 16) as usize,
        }
    }
}

enum Direction {
    Receive = 0,
    Transmit = 2,
}

pub struct Device {
    buffer: [u32; 16],
    configuration: u8,
    requests: u32,
    errors: u32,
    resets: u32,
    dfu: dfu::Transfer,
    pending_out: usize,
    storage: Storage,
    reboot: bool,
}

impl Device {
    pub fn init() -> Option<Self> {
        diag(0, 0x4d30_5553);
        diag(1, 1);
        unsafe {
            // Supply the USB divider without changing the Wi-Fi PLL root.
            update32(CLOCK_GATE, 0, 1 << 13);
            update32(USB_PLL, 0, (1 << 29) | (1 << 28));
            delay(2048);
            update32(USB_PLL, 1 << 28, 0);
            delay(2048);
            update32(USB_PLL, 0, 1 << 28);
            diag(1, 2);

            // PHY power, reset and device-role selection remain in MCU PDS.
            update32(PHY_CONTROL, 3 << 2, (1 << 6) | 1);
            delay(1024);
            update32(PDS_CONTROL, 1, 1 << 5);
            delay(1024);
            update32(PDS_CONTROL, 0, 1 << 1);
            delay(3 * 1024 * 1000);
            update32(PDS_CONTROL, 0, 1);
            delay(2 * 1024 * 1000);
            diag(1, 3);

            update32(PHY_TEST, 0, 1); // detach until ready
            write32(CONTROL, CHIP_ENABLE | FULL_SPEED | SOFT_RESET);
            let mut ready = false;
            for _ in 0..POLL_LIMIT {
                if read32(CONTROL) & SOFT_RESET == 0 {
                    ready = true;
                    break;
                }
            }
            if !ready {
                diag(1, 0xdead_0003);
                return None;
            }
            write32(ADDRESS, 0);
            write32(SOF_TIMER, 0x2710);
            write32(CX_FIFO, CX_CLEAR);
            write32(MASK_GROUP, !5);
            write32(MASK0, !(CX_SETUP | CX_OUT | CX_ABORT | CX_FAIL));
            write32(MASK1, u32::MAX);
            write32(MASK2, !7);
            write32(EVENTS0, CX_ABORT | CX_FAIL);
            write32(EVENTS2, u32::MAX);
            write32(DMA_FIFO, 0);
            write32(DMA_EVENTS, u32::MAX);
            write32(DMA_ENABLE, 1);
            // MIE/mie remain disabled: service raw flags from the polling loop.
            write32(BASE + 0xc4, 6); // only the device group is unmasked
            write32(CONTROL, CHIP_ENABLE | FULL_SPEED | (1 << 2));
            update32(PHY_TEST, 1, 0);
            diag(1, 4);
        }
        Some(Self {
            buffer: [0; 16],
            configuration: 0,
            requests: 0,
            errors: 0,
            resets: 0,
            dfu: dfu::Transfer::new(),
            pending_out: 0,
            storage: Storage::new(),
            reboot: false,
        })
    }

    pub fn poll(&mut self) {
        if self.reboot {
            // Let EP0's zero-length status packet finish before disconnecting.
            // This conservative CPU delay is not a calibrated time source.
            delay(32_000_000);
            unsafe { update32(PHY_TEST, 0, 1) };
            board::reboot();
        }
        unsafe {
            let events = read32(EVENTS2);
            if events & BUS_RESET != 0 {
                Self::reset_dma();
                write32(DMA_FIFO, 0);
                write32(ADDRESS, 0);
                write32(CX_FIFO, CX_CLEAR);
                self.configuration = 0;
                self.reset_session();
                self.resets = self.resets.wrapping_add(1);
                diag(5, self.resets);
            }
            write32(EVENTS2, events & 7);
            let events = read32(EVENTS0);
            if events & CX_ABORT != 0 {
                write32(EVENTS0, CX_ABORT);
                self.cancel_out();
                return;
            }
            if events & CX_FAIL != 0 {
                write32(EVENTS0, CX_FAIL);
            }
            if events & CX_SETUP != 0 {
                self.cancel_out();
                write32(DMA_FIFO, 1 << 4);
                let first = read32(SETUP_DATA);
                let second = read32(SETUP_DATA);
                write32(DMA_FIFO, 0);
                self.requests = self.requests.wrapping_add(1);
                diag(2, first);
                diag(3, second);
                diag(4, self.requests);
                self.setup(first, second);
            } else if events & CX_OUT != 0 && self.pending_out != 0 {
                self.receive();
            }
        }
    }

    fn setup(&mut self, first: u32, second: u32) {
        let setup = Setup::decode(first, second);
        match setup.kind & 0x60 {
            0 => self.standard_request(setup),
            0x20 => self.dfu_request(setup),
            0x40 => self.vendor_request(setup),
            _ => self.stall(),
        }
    }

    fn standard_request(&mut self, setup: Setup) {
        let Setup {
            kind,
            request,
            value,
            index,
            length,
        } = setup;
        match (kind, request) {
            (0x80, 6) => {
                let descriptor = match (value >> 8, value as u8, index) {
                    (1, 0, 0) => Some(DEVICE),
                    (2, 0, 0) => Some(CONFIG),
                    (3, 0, 0) => Some(LANG),
                    (3, 1, 0 | 0x0409) => Some(PRODUCT),
                    (3, 2, 0 | 0x0409) => Some(NOR_TARGET),
                    (3, 3, 0 | 0x0409) => Some(SD_TARGET),
                    _ => None,
                };
                if let Some(data) = descriptor {
                    self.reply(data, length);
                } else {
                    self.stall();
                }
            }
            (0, 5) if value < 128 && index == 0 && length == 0 => unsafe {
                write32(ADDRESS, u32::from(value));
                self.done();
            },
            (0, 9) if value <= 1 && index == 0 && length == 0 => unsafe {
                self.configuration = value as u8;
                self.reset_session();
                update32(ADDRESS, 1 << 7, u32::from(value) << 7);
                self.done();
            },
            (0x80, 8) if value == 0 && index == 0 && length == 1 => {
                self.reply(&[self.configuration], length);
            }
            (0x80, 0) if value == 0 && index == 0 && length == 2 => {
                self.reply(&[0, 0], length);
            }
            (0x81, 0 | 10) if self.configuration == 1 && value == 0 && index == 0 => {
                let data = [
                    if request == 10 {
                        self.storage.target() as u8
                    } else {
                        0
                    },
                    0,
                ];
                if length == if request == 0 { 2 } else { 1 } {
                    self.reply(&data[..length], length);
                } else {
                    self.stall();
                }
            }
            (1, 11)
                if self.configuration == 1
                    && value <= 1
                    && index == 0
                    && length == 0
                    && self.dfu.can_select_alternate() =>
            {
                let target = Target::from(value).unwrap();
                self.dfu
                    .select_alternate(self.storage.target() as u8, target as u8);
                self.storage.select_target(target);
                self.pending_out = 0;
                self.done();
            }
            _ => self.stall(),
        }
    }

    fn dfu_request(&mut self, setup: Setup) {
        let Setup {
            kind,
            request,
            value,
            index,
            length,
        } = setup;
        match (kind, request) {
            (0xa1, 3) if self.configuration == 1 && value == 0 && index == 0 && length == 6 => {
                // Only a complete, length-checked SRAM transaction can commit.
                if let Some(data) = self.dfu.manifest_data() {
                    if let Err(status) = self.storage.write(data) {
                        self.dfu.storage_error(status);
                    }
                }
                let status = self.dfu.get_status();
                self.reply(&status, length);
            }
            (0xa1, 5) if self.configuration == 1 && value == 0 && index == 0 && length == 1 => {
                self.reply(&[self.dfu.state() as u8], length);
            }
            (0x21, 1) if self.configuration == 1 && index == 0 => {
                if !self.storage.writable() {
                    self.stall(); // protected/uninitialized target, no data reception or write
                } else if self.dfu.download(value, length).is_err() {
                    self.stall();
                } else if length == 0 {
                    self.done();
                } else {
                    self.pending_out = length; // wait for the actual EP0 OUT packet
                }
            }
            (0xa1, 2) if self.configuration == 1 && index == 0 => {
                if value == 0 && self.dfu.is_idle() {
                    let Some(count) = self.storage.length() else {
                        self.stall();
                        return;
                    };
                    let loaded = self
                        .dfu
                        .load_readback(count, |data| self.storage.read(data).is_ok());
                    if !loaded {
                        self.stall();
                        return;
                    }
                }
                match self.dfu.upload(value, length) {
                    Ok((data, count)) => self.reply(&data[..count], length),
                    Err(()) => self.stall(),
                }
            }
            (0x21, 4 | 6) if self.configuration == 1 && value == 0 && index == 0 && length == 0 => {
                let result = if request == 4 {
                    self.dfu.clear_error()
                } else {
                    self.dfu.abort()
                };
                if result.is_ok() {
                    self.done();
                } else {
                    self.stall();
                }
            }
            // Lab DFU-mode exit: DETACH requests a whole-chip normal reboot.
            // Never accept it in the middle of a download or failed commit.
            (0x21, 0)
                if self.configuration == 1
                    && index == 0
                    && length == 0
                    && self.pending_out == 0
                    && !self.storage.failed()
                    && self.dfu.can_detach() =>
            {
                if let Err(status) = self.storage.prepare_reset() {
                    self.dfu.storage_error(status);
                    self.stall();
                    return;
                }
                self.done();
                self.reboot = true;
            }
            _ => self.stall(),
        }
    }

    fn vendor_request(&mut self, setup: Setup) {
        let Setup {
            kind,
            request,
            value,
            index,
            length,
        } = setup;
        match (kind, request) {
            (0xc0, 1) if self.configuration == 1 && value == 0 && index == 0 => {
                let mut status = [0u8; 16];
                status[..4].copy_from_slice(b"M0US");
                status[4] = 2; // incompatible storage protocol, little-endian u16
                status[6] = self.configuration;
                status[7] = u8::from(board::d0_is_held());
                status[8..12].copy_from_slice(&self.requests.to_le_bytes());
                status[12..].copy_from_slice(&self.errors.to_le_bytes());
                self.reply(&status, length);
            }
            (0x40, 2)
                if self.configuration == 1
                    && index == 0
                    && length == 0
                    && self.pending_out == 0
                    && self.dfu.is_idle()
                    && value <= 1 =>
            {
                match self.storage.enter(Target::from(value).unwrap()) {
                    Ok(()) => self.done(),
                    Err(status) => {
                        self.dfu.storage_error(status);
                        self.stall();
                    }
                }
            }
            (0xc0, 3) if self.configuration == 1 && value == 0 && index == 0 && length == 32 => {
                self.reply(&flash::diagnostics(), length);
            }
            (0x40, 4)
                if self.configuration == 1
                    && length == 0
                    && self.pending_out == 0
                    && self.dfu.is_idle() =>
            {
                if self.storage.select_nor(value, index).is_ok() {
                    self.dfu = dfu::Transfer::new();
                    self.done();
                } else {
                    self.stall();
                }
            }
            (0xc0, 4) if self.configuration == 1 && value == 0 && index == 0 && length == 16 => {
                let mut data = [0; 16];
                data[..4].copy_from_slice(b"M0SL");
                data[4..6].copy_from_slice(&2u16.to_le_bytes());
                let (slot, index, address) = self.storage.nor_selection();
                data[6..8].copy_from_slice(&slot.to_le_bytes());
                data[8..12].copy_from_slice(&u32::from(index).to_le_bytes());
                data[12..].copy_from_slice(&address.to_le_bytes());
                self.reply(&data, length);
            }
            (0x40, 5)
                if self.configuration == 1
                    && length == 0
                    && self.pending_out == 0
                    && self.dfu.is_idle()
                    && matches!((value, index), (0x4d30, 0x5550) | (0, 0)) =>
            {
                self.storage.allow_m0(value != 0);
                self.dfu = dfu::Transfer::new();
                self.done();
            }
            (0xc0, 5) if self.configuration == 1 && value == 0 && index == 0 && length == 1 => {
                self.reply(&[u8::from(self.storage.m0_allowed())], length);
            }
            // 32-bit LBA in wValue/wIndex; request encodes 1..8 blocks.
            (0x40, 0x10..=0x17)
                if self.configuration == 1
                    && length == 0
                    && self.pending_out == 0
                    && self.dfu.is_idle() =>
            {
                self.dfu = dfu::Transfer::new();
                let lba = u32::from(value) | (u32::from(index) << 16);
                match self.storage.select_sd(lba, u16::from(request - 0x0f)) {
                    Ok(()) => self.done(),
                    Err(status) => {
                        self.dfu.storage_error(status);
                        self.stall();
                    }
                }
            }
            (0xc0, 6) if self.configuration == 1 && value == 0 && index == 0 && length == 16 => {
                let mut data = [0u8; 16];
                data[..4].copy_from_slice(b"M0SD");
                data[4..6].copy_from_slice(&2u16.to_le_bytes());
                if let Some(range) = self.storage.sd_range() {
                    data[6..8].copy_from_slice(&range.blocks.to_le_bytes());
                    data[8..12].copy_from_slice(&range.lba.to_le_bytes());
                    data[12..].copy_from_slice(&(range.len() as u32).to_le_bytes());
                }
                self.reply(&data, length);
            }
            (0xc0, 7) if self.configuration == 1 && value == 0 && index == 0 && length == 48 => {
                if let Some(card) = self.storage.card_info() {
                    let mut data = [0u8; 48];
                    data[..4].copy_from_slice(b"M0SI");
                    data[4..6].copy_from_slice(&2u16.to_le_bytes());
                    data[6..8].copy_from_slice(&512u16.to_le_bytes());
                    data[8..16].copy_from_slice(&card.blocks.to_le_bytes());
                    data[16..32].copy_from_slice(&card.cid);
                    data[32..].copy_from_slice(&card.csd);
                    self.reply(&data, length);
                } else {
                    self.stall();
                }
            }
            (0xc0, 8) if self.configuration == 1 && value == 0 && index == 0 && length == 32 => {
                self.reply(&sd::diagnostics(), length);
            }
            (0xc0, 9) if self.configuration == 1 && value == 0 && index == 0 && length == 64 => {
                self.reply(&sd::registers(), length);
            }
            _ => self.stall(),
        }
    }

    fn reset_session(&mut self) {
        self.storage.select_target(Target::Nor);
        self.pending_out = 0;
        self.dfu = dfu::Transfer::new();
        self.storage.allow_m0(false);
    }

    fn cancel_out(&mut self) {
        if self.pending_out != 0 {
            self.pending_out = 0;
            self.dfu.transport_error();
        }
    }

    fn reset_dma() {
        unsafe {
            write32(DMA_ENABLE, 0);
            write32(DMA_CHANNEL, 0);
            write32(DMA_ENABLE, 1);
        }
    }

    /// Both directions use the controller's existing EP0 DMA engine.
    fn dma(&mut self, count: usize, direction: Direction) -> bool {
        unsafe {
            write32(DMA_EVENTS, DMA_COMPLETE | DMA_ERROR);
            write32(DMA_ADDRESS, self.buffer.as_mut_ptr() as u32);
            write32(DMA_CHANNEL, ((count as u32) << 8) | direction as u32 | 1);
            for _ in 0..POLL_LIMIT {
                if read32(DMA_EVENTS) & DMA_ERROR != 0 || read32(EVENTS2) & BUS_RESET != 0 {
                    break;
                }
                if read32(DMA_CHANNEL) & 1 == 0 {
                    write32(DMA_EVENTS, DMA_COMPLETE);
                    return true;
                }
            }
        }
        false
    }

    fn dma_error(&mut self) {
        diag(7, unsafe { read32(DMA_EVENTS) });
        Self::reset_dma();
        self.errors = self.errors.wrapping_add(1);
        diag(6, self.errors);
    }

    fn receive(&mut self) {
        let count = self.pending_out;
        self.pending_out = 0;
        let actual = (unsafe { read32(CX_FIFO) } >> 24) & 0x7f;
        if actual as usize == count && self.dma(count, Direction::Receive) {
            let mut data = [0; dfu::PACKET];
            for (i, byte) in data[..count].iter_mut().enumerate() {
                *byte = unsafe { self.buffer.as_ptr().cast::<u8>().add(i).read_volatile() };
            }
            self.dfu.accept(&data[..count]);
            self.done();
            return;
        }
        self.dma_error();
        unsafe { write32(CX_FIFO, CX_CLEAR) };
        self.dfu.transport_error();
        self.stall();
    }

    fn reply(&mut self, data: &[u8], requested: usize) {
        let count = data.len().min(requested);
        if count == 0 {
            self.done();
            return;
        }
        // All current descriptors and replies fit one 64-byte EP0 packet.
        if count > dfu::PACKET {
            self.stall();
            return;
        }
        for (word, bytes) in self.buffer.iter_mut().zip(data[..count].chunks(4)) {
            let mut padded = [0; 4];
            padded[..bytes.len()].copy_from_slice(bytes);
            *word = u32::from_le_bytes(padded);
        }
        if self.dma(count, Direction::Transmit) {
            self.done();
        } else {
            self.dma_error();
            self.stall();
        }
    }

    fn done(&self) {
        unsafe { update32(CX_FIFO, 0, CX_DONE) }
    }

    fn stall(&self) {
        unsafe { update32(CX_FIFO, 0, CX_STALL) }
    }
}
