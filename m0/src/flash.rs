//! M0 SRAM-only, polled W25Q128 command access. No chip erase or status writes.
//!
//! Fixed image slots, transferred one erase unit at a time. M0 writes are opt-in.
//! D0 stays held until system reset; live XIP resume is not implemented here.

use crate::{board, regs::*};

pub const SECTOR: usize = 4096;
pub const SCRATCH: u32 = 0x0006_0000;

/// The host chooses a slot and relative erase-unit index, never an address.
/// IDs 7/8 allow auditing the protected firmware without allowing writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sector {
    address: u32,
    writable: bool,
}

impl Sector {
    pub fn new(slot: u16, index: u16) -> Option<Self> {
        let (start, end, writable) = match slot {
            0 => (SCRATCH, SCRATCH + SECTOR as u32, true),
            1 => (0x52000, 0xa2000, true),   // D0 body
            2 => (0x1000, 0x2000, true),     // Group1 metadata
            3 => (0xd2000, 0xe2000, true),   // Boot DTB
            4 => (0xf1000, 0xf2000, true),   // U-Boot length header at +0xff8
            5 => (0xf2000, 0x552000, true),  // U-Boot body
            6 => (0x852000, 0x8d2000, true), // OpenSBI, including declared BSS
            7 => (0, 0x1000, false),         // Group0 metadata
            8 => (0x2000, 0x52000, false),   // Entire M0 slot
            _ => return None,
        };
        let address = start + u32::from(index) * SECTOR as u32;
        (address < end).then_some(Self { address, writable })
    }

    pub fn address(self) -> u32 {
        self.address
    }

    /// Temporary development escape hatch, enabled explicitly over USB.
    pub fn for_update(slot: u16, index: u16, experimental_m0: bool) -> Option<Self> {
        let mut sector = Self::new(slot, index)?;
        if experimental_m0 && matches!(slot, 7 | 8) {
            sector.writable = true;
        }
        Some(sector)
    }

    pub fn writable(self) -> bool {
        self.writable
    }
}
const BASE: usize = 0x2000_b000;
const OWNER: usize = BASE + 4;
const COMMAND: usize = BASE + 8;
const BUFFER: usize = BASE + 0x600;
const EXPECTED_ID: u32 = 0x0018_40ef;
const COMMAND_POLLS: usize = 65536;
const NOR_POLLS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    State,
    ControllerTimeout,
    BusyTimeout,
    Chip,
    WriteEnable,
    Erase,
    Verify,
    Bounds,
}

pub struct Nor {
    pub jedec: u32,
}

#[unsafe(no_mangle)]
static mut FLASH_DIAG: [u32; 8] = [0; 8];

fn mark(index: usize, value: u32) {
    unsafe {
        core::ptr::addr_of_mut!(FLASH_DIAG)
            .cast::<u32>()
            .add(index)
            .write_volatile(value)
    }
}

pub fn record_error(error: Error) {
    mark(6, error as u32 + 1);
}

pub fn diagnostics() -> [u8; 32] {
    let mut bytes = [0; 32];
    for (i, word) in bytes.chunks_exact_mut(4).enumerate() {
        let value = unsafe {
            core::ptr::addr_of!(FLASH_DIAG)
                .cast::<u32>()
                .add(i)
                .read_volatile()
        };
        word.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

impl Nor {
    pub fn init() -> Result<Self, Error> {
        mark(0, 0x4c46_304d); // M0FL
        mark(1, 1);
        if !board::d0_is_held() {
            return Err(Error::State);
        }
        unsafe {
            // IF2 ownership and 32-bit-address operation need separate support.
            if read32(BASE + 0xc4) & (3 << 28) == (3 << 28) || read32(BASE) & (1 << 19) != 0 {
                return Err(Error::State);
            }
            let original_owner = read32(OWNER);
            mark(3, original_owner);
            Self::wait_controller()?;
            write32(OWNER, original_owner & !((1 << 28) | (1 << 30)));
            mark(4, read32(OWNER));
            mark(1, 2);
            let mut nor = Self { jedec: 0 };
            // Match the board's continuous-read exit: four 0xff bytes on one line.
            nor.command(0xff, Some(0x00ff_ffff), false, 0)?;
            nor.command(0x9f, None, false, 3)?;
            nor.jedec = read32(BUFFER) & 0x00ff_ffff;
            mark(2, nor.jedec);
            if nor.jedec != EXPECTED_ID {
                return Err(Error::Chip);
            }
            nor.ready()?;
            mark(1, 3);
            Ok(nor)
        }
    }

    fn wait_controller() -> Result<(), Error> {
        for _ in 0..COMMAND_POLLS {
            if unsafe { read32(COMMAND) } & 1 == 0 {
                return Ok(());
            }
        }
        Err(Error::ControllerTimeout)
    }

    fn command(
        &mut self,
        opcode: u8,
        address: Option<u32>,
        write: bool,
        length: usize,
    ) -> Result<(), Error> {
        if !board::d0_is_held() || length > 256 || address.is_some_and(|v| v > 0x00ff_ffff) {
            return Err(Error::Bounds);
        }
        Self::wait_controller()?;
        let mut control = 1 << 27; // command enabled, one byte; single-line SPI
        if address.is_some() {
            control |= (1 << 26) | (2 << 17);
        }
        if length != 0 {
            control |= (1 << 24) | (((length - 1) as u32) << 2);
        }
        if write {
            control |= 1 << 23;
        }
        unsafe {
            write32(COMMAND, 0); // clear trigger before updating command words
            write32(
                BASE + 0x0c,
                (u32::from(opcode) << 24) | address.unwrap_or(0),
            );
            write32(BASE + 0x10, 0);
            write32(COMMAND, control);
            write32(COMMAND, control | 2);
        }
        let result = Self::wait_controller();
        mark(5, unsafe { read32(COMMAND) });
        result
    }

    pub fn status(&mut self) -> Result<u8, Error> {
        self.command(0x05, None, false, 1)?;
        let status = unsafe { read32(BUFFER) } as u8;
        mark(7, u32::from(status));
        Ok(status)
    }

    fn ready(&mut self) -> Result<(), Error> {
        for _ in 0..NOR_POLLS {
            if self.status()? & 1 == 0 {
                return Ok(());
            }
            delay(16);
        }
        Err(Error::BusyTimeout)
    }

    fn write_enable(&mut self) -> Result<(), Error> {
        self.ready()?;
        self.command(0x06, None, false, 0)?;
        if self.status()? & 2 == 0 {
            return Err(Error::WriteEnable);
        }
        Ok(())
    }

    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), Error> {
        self.ready()?;
        for (index, chunk) in data.chunks_mut(256).enumerate() {
            self.command(
                0x03,
                Some(address + (index * 256) as u32),
                false,
                chunk.len(),
            )?;
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte = (unsafe { read32(BUFFER + (i & !3)) } >> ((i & 3) * 8)) as u8;
            }
        }
        Ok(())
    }

    pub fn read_selected(&mut self, sector: Sector, data: &mut [u8]) -> Result<(), Error> {
        if data.len() != SECTOR {
            return Err(Error::Bounds);
        }
        self.read(sector.address, data)
    }

    pub fn prepare_reset(&mut self) -> Result<(), Error> {
        self.ready()?;
        self.command(0x04, None, false, 0)
    }

    pub fn replace_sector(&mut self, sector: Sector, data: &[u8]) -> Result<(), Error> {
        if !sector.writable || data.len() != SECTOR || self.jedec != EXPECTED_ID {
            return Err(Error::Bounds);
        }
        let start = sector.address;
        mark(1, 10);
        self.write_enable()?;
        self.command(0x20, Some(start), false, 0)?;
        self.ready()?;
        mark(1, 11);
        let mut verify = [0; 256];
        for address in (start..start + SECTOR as u32).step_by(256) {
            self.read(address, &mut verify)?;
            if verify.iter().any(|&byte| byte != 0xff) {
                return Err(Error::Erase);
            }
        }
        for (index, page) in data.chunks_exact(256).enumerate() {
            mark(1, 100 + index as u32);
            self.write_enable()?;
            for (i, word) in page.chunks_exact(4).enumerate() {
                unsafe { write32(BUFFER + i * 4, u32::from_le_bytes(word.try_into().unwrap())) };
            }
            self.command(0x02, Some(start + (index * 256) as u32), true, page.len())?;
            self.ready()?;
        }
        self.command(0x04, None, false, 0)?; // leave WEL cleared
        for (index, expected) in data.chunks_exact(256).enumerate() {
            self.read(start + (index * 256) as u32, &mut verify)?;
            if verify != expected {
                return Err(Error::Verify);
            }
        }
        mark(1, 200);
        Ok(())
    }
}
