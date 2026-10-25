//! BL808 SDH, polling/PIO only. SDHC/SDXC, 512-byte CMD17/CMD24.
//! Register layout: SDK bl808_driver/regs/sdh_reg.h; board pads/clock:
//! experiments/archive/uboot-sd and U-Boot's bflb clock/pinctrl drivers.
use crate::{board, regs::*};

pub const BLOCK: usize = 512;
const BASE: usize = 0x2006_0000;
const LIMIT: usize = 200_000;
const R1_ERRORS: u32 = 0xfdff_e008;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    State,
    Bounds,
    Timeout,
    Controller,
    Unsupported,
    Card,
    Verify,
}

#[unsafe(no_mangle)]
static mut SD_DIAG: [u32; 8] = [0; 8];
fn mark(index: usize, value: u32) {
    unsafe {
        core::ptr::addr_of_mut!(SD_DIAG)
            .cast::<u32>()
            .add(index)
            .write_volatile(value);
    }
}
pub fn record_error(error: Error) {
    mark(7, error as u32 + 1);
}
pub fn diagnostics() -> [u8; 32] {
    let mut bytes = [0; 32];
    for (i, word) in bytes.chunks_exact_mut(4).enumerate() {
        word.copy_from_slice(
            &unsafe {
                core::ptr::addr_of!(SD_DIAG)
                    .cast::<u32>()
                    .add(i)
                    .read_volatile()
            }
            .to_le_bytes(),
        );
    }
    bytes
}

/// A bounded transaction; addition uses u64 so the final 32-bit LBA is usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub lba: u32,
    pub blocks: u16,
}
impl Range {
    pub fn new(lba: u32, blocks: u16, capacity: u64) -> Result<Self, Error> {
        if !(1..=8).contains(&blocks) || u64::from(lba) + u64::from(blocks) > capacity {
            return Err(Error::Bounds);
        }
        Ok(Self { lba, blocks })
    }
    pub fn len(self) -> usize {
        usize::from(self.blocks) * BLOCK
    }
}

pub struct Card {
    pub blocks: u64,
    pub cid: [u8; 16],
    pub csd: [u8; 16],
    rca: u32,
}

// SDK accesses the packed host/power and timeout/reset registers as u16.
// Use aligned halfword accesses on the MCU peripheral bridge as well.
unsafe fn r8(o: usize) -> u8 {
    unsafe { (read16(BASE + (o & !1)) >> ((o & 1) * 8)) as u8 }
}
unsafe fn r16(o: usize) -> u16 {
    unsafe { read16(BASE + o) }
}
unsafe fn w8(o: usize, v: u8) {
    let shift = (o & 1) * 8;
    unsafe {
        let a = BASE + (o & !1);
        write16(a, (read16(a) & !(0xff << shift)) | (u16::from(v) << shift));
    }
}
unsafe fn w16(o: usize, v: u16) {
    unsafe { write16(BASE + o, v) }
}
pub fn registers() -> [u8; 64] {
    let mut result = [0; 64];
    let addresses = [
        0x20000430,
        0x20000830,
        0x20000588,
        BASE + 0x2c,
        BASE + 0x28,
        BASE + 0x118,
        BASE + 0x114,
        BASE + 0x34,
        0x200008c4,
        0x200008c8,
        0x200008cc,
        0x200008d0,
        0x200008d4,
        0x200008d8,
        BASE + 0x0c,
        BASE + 8,
    ];
    for (chunk, address) in result.chunks_exact_mut(4).zip(addresses) {
        chunk.copy_from_slice(&unsafe { read32(address) }.to_le_bytes());
    }
    result
}
fn wait(mut predicate: impl FnMut() -> Result<bool, Error>) -> Result<(), Error> {
    for _ in 0..LIMIT {
        if predicate()? {
            return Ok(());
        }
        delay(32);
    }
    Err(Error::Timeout)
}
fn event(mask: u32) -> Result<(), Error> {
    wait(|| {
        let s = unsafe { read32(BASE + 0x30) };
        if s & 0xffff_8000 != 0 {
            Err(Error::Controller)
        } else {
            Ok(s & mask == mask)
        }
    })
}
fn command(cmd: u8, arg: u32, flags: u16, mode: u16) -> Result<u32, Error> {
    if !board::d0_is_held() {
        return Err(Error::State);
    }
    wait(|| Ok(unsafe { read32(BASE + 0x24) } & 3 == 0))?;
    unsafe {
        write32(BASE + 0x30, u32::MAX);
        write32(BASE + 8, arg);
        if flags & 0x20 != 0 {
            w16(0x0c, mode); // DMA/multiblock/auto-CMD12 always disabled
        }
        w16(0x0e, (u16::from(cmd) << 8) | flags);
    }
    mark(2, u32::from(cmd));
    mark(3, arg);
    let result = event(1);
    mark(4, unsafe { read32(BASE + 0x30) });
    mark(5, unsafe { read32(BASE + 0x24) });
    mark(6, unsafe { read32(BASE + 0x10) });
    result?;
    unsafe {
        write32(BASE + 0x30, 1);
    }
    Ok(unsafe { read32(BASE + 0x10) })
}
fn r1(cmd: u8, arg: u32, flags: u16, mode: u16) -> Result<u32, Error> {
    let response = command(cmd, arg, flags, mode)?;
    if response & R1_ERRORS != 0 {
        return Err(Error::Card);
    }
    Ok(response)
}
fn long_response() -> [u8; 16] {
    let mut value = 0u128;
    for i in (0..4).rev() {
        value = (value << 32) | u128::from(unsafe { read32(BASE + 0x10 + i * 4) });
    }
    (value << 8).to_be_bytes() // SDH strips CRC/end bit from R2
}
pub fn capacity(csd: [u8; 16]) -> Result<u64, Error> {
    let value = u128::from_be_bytes(csd);
    if value >> 126 != 1 || (value >> 80) & 15 != 9 || (value >> 22) & 15 != 9 {
        return Err(Error::Unsupported);
    }
    Ok((((value >> 48) & 0x3f_ffff) as u64 + 1) * 1024)
}
impl Card {
    pub fn init() -> Result<Self, Error> {
        // The board also needs a second U-Boot rescan after software POR.
        // Retry controller/card identification once, before exposing any I/O.
        // An absent card still returns an error; no data command is issued here.
        match Self::initialize() {
            Err(Error::Controller | Error::Timeout) => Self::initialize(),
            result => result,
        }
    }
    fn initialize() -> Result<Self, Error> {
        mark(0, 0x4453_304d);
        mark(1, 1);
        mark(7, 0);
        if !board::d0_is_held() {
            return Err(Error::State);
        }
        unsafe {
            // Enable existing 96 MHz Wi-Fi PLL /5 output only. Never retune its root.
            update32(0x2000_0830, 0, 1 << 2);
            update32(0x2000_0588, 0, 1 << 22);
            update32(0x2000_0430, 1 << 13, 0);
            update32(0x2000_0430, (7 << 9) | (1 << 12), 0);
            update32(0x2000_0430, 0, 1 << 13);
            write32(BASE + 0x38, 0); // no interrupt delivery, including inherited DMA
            w8(0x2f, 1); // reset all aborts any inherited data/DMA transfer
        }
        wait(|| Ok(unsafe { r8(0x2f) } & 1 == 0))?;
        unsafe {
            write32(BASE, 0);
            write32(BASE + 0x58, 0);
            write32(BASE + 0x5c, 0);
            write32(BASE + 0x38, 0);
            write32(BASE + 0x34, 0x007f_8033); // polled command/data/error flags
            write32(BASE + 0x30, u32::MAX);
            for pin in 0..6 {
                update32(0x2000_08c4 + pin * 4, 0xc000_1fff, 0x17);
            }
            w8(0x28, 0); // 1 bit, normal speed, SDMA selection but DMA never enabled
            w8(0x29, 0x0f); // 3.3 V
            w8(0x2e, 0x0e);
            // SDK SDH_Ctrl_Init selects the internal transmit clock.
            update32(BASE + 0x118, 0, 1 << 30);
            // /960 of nominal 96 MHz = 100 kHz, matching verified profile.
            w16(0x2c, 0xe041);
        }
        wait(|| Ok(unsafe { r16(0x2c) } & 2 != 0))?;
        unsafe {
            w16(0x2c, 0xe045);
        }
        delay(3_200_000); // >=74 initial clocks, conservative for current MCU clocks
        mark(1, 2);
        command(0, 0, 0, 0)?;
        delay(320_000); // card reset recovery before CMD8 (also used by U-Boot)
        if command(8, 0x1aa, 0x1a, 0)? & 0xfff != 0x1aa {
            return Err(Error::Unsupported);
        }
        mark(1, 3);
        let mut ready = false;
        for _ in 0..1000 {
            r1(55, 0, 0x1a, 0)?;
            let ocr = command(41, 0x40ff_8000, 2, 0)?;
            if ocr & (1 << 31) != 0 {
                if ocr & (1 << 30) == 0 || ocr & 0x00ff_8000 == 0 {
                    return Err(Error::Unsupported);
                }
                ready = true;
                break;
            }
            delay(32_000);
        }
        if !ready {
            return Err(Error::Timeout);
        }
        mark(1, 4);
        command(2, 0, 9, 0)?;
        let cid = long_response();
        let response = command(3, 0, 0x1a, 0)?;
        let rca = response & 0xffff_0000;
        if rca == 0 || response & 0xe000 != 0 {
            return Err(Error::Card);
        }
        command(9, rca, 9, 0)?;
        let csd = long_response();
        let blocks = capacity(csd)?;
        r1(7, rca, 0x1b, 0)?;
        event(2)?;
        unsafe {
            write32(BASE + 0x30, 2);
        }
        let mut card = Self {
            blocks,
            cid,
            csd,
            rca,
        };
        card.ready()?;
        mark(1, 5);
        Ok(card)
    }
    fn ready(&mut self) -> Result<(), Error> {
        wait(|| Ok(unsafe { read32(BASE + 0x24) } & ((1 << 20) | 3) == 1 << 20))?;
        // A high DAT0 alone does not establish successful programming.
        for _ in 0..1000 {
            let status = r1(13, self.rca, 0x1a, 0)?;
            if status & (1 << 8) != 0 && (status >> 9) & 15 == 4 {
                return Ok(());
            }
            delay(32_000);
        }
        Err(Error::Timeout)
    }
    fn validate(&self, range: Range, length: usize) -> Result<(), Error> {
        Range::new(range.lba, range.blocks, self.blocks)?;
        if range.len() != length {
            return Err(Error::Bounds);
        }
        Ok(())
    }
    fn read_block(&mut self, lba: u32, data: &mut [u8]) -> Result<(), Error> {
        self.ready()?;
        unsafe {
            w16(4, 512);
            w16(6, 1);
        }
        r1(17, lba, 0x3a, 0x10)?;
        event(1 << 5)?;
        wait(|| Ok(unsafe { read32(BASE + 0x24) } & (1 << 11) != 0))?;
        for word in data.chunks_exact_mut(4) {
            word.copy_from_slice(&unsafe { read32(BASE + 0x20) }.to_le_bytes());
        }
        unsafe {
            write32(BASE + 0x30, 1 << 5);
        }
        event(2)?;
        unsafe {
            write32(BASE + 0x30, 2);
        }
        self.ready()
    }
    pub fn read(&mut self, range: Range, data: &mut [u8]) -> Result<(), Error> {
        self.validate(range, data.len())?;
        for (i, block) in data.chunks_exact_mut(BLOCK).enumerate() {
            self.read_block(range.lba + i as u32, block)?;
        }
        Ok(())
    }
    pub fn write(&mut self, range: Range, data: &[u8]) -> Result<(), Error> {
        self.validate(range, data.len())?; // before any MMIO, including ready
        let mut verify = [0u8; BLOCK];
        for (i, block) in data.chunks_exact(BLOCK).enumerate() {
            let lba = range.lba + i as u32;
            self.ready()?;
            unsafe {
                w16(4, 512);
                w16(6, 1);
            }
            r1(24, lba, 0x3a, 0)?;
            event(1 << 4)?;
            wait(|| Ok(unsafe { read32(BASE + 0x24) } & (1 << 10) != 0))?;
            for word in block.chunks_exact(4) {
                unsafe {
                    write32(BASE + 0x20, u32::from_le_bytes(word.try_into().unwrap()));
                }
            }
            unsafe {
                write32(BASE + 0x30, 1 << 4);
            }
            event(2)?;
            unsafe {
                write32(BASE + 0x30, 2);
            }
            self.ready()?;
            self.read_block(lba, &mut verify)?;
            if verify != block {
                return Err(Error::Verify);
            }
        }
        Ok(())
    }
    pub fn prepare_reset(&mut self) -> Result<(), Error> {
        self.ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geometry_and_end_of_card_do_not_wrap() {
        assert_eq!(Range::new(u32::MAX, 1, 1u64 << 32).unwrap().len(), 512);
        assert_eq!(Range::new(u32::MAX, 2, 1u64 << 32), Err(Error::Bounds));
        for count in [0, 9, u16::MAX] {
            assert_eq!(Range::new(0, count, 1024), Err(Error::Bounds));
        }
        assert_eq!(Range::new(1023, 2, 1024), Err(Error::Bounds));
        assert_eq!(
            capacity(((1u128 << 126) | (9 << 80) | (9 << 22)).to_be_bytes()),
            Ok(1024)
        );
        assert_eq!(
            capacity(((1u128 << 126) | (9 << 80) | (9 << 22) | (0x3fffffu128 << 48)).to_be_bytes()),
            Ok(1u64 << 32)
        );
        assert_eq!(capacity([0; 16]), Err(Error::Unsupported));
    }
    #[test]
    fn malformed_transfers_rejected_before_mmio() {
        let mut card = Card {
            blocks: 100,
            cid: [0; 16],
            csd: [0; 16],
            rca: 1 << 16,
        };
        for range in [
            Range {
                lba: 100,
                blocks: 1,
            },
            Range { lba: 99, blocks: 2 },
            Range { lba: 0, blocks: 0 },
            Range { lba: 0, blocks: 9 },
        ] {
            assert_eq!(card.read(range, &mut [0; 512]), Err(Error::Bounds));
            assert_eq!(card.write(range, &[0; 512]), Err(Error::Bounds));
        }
        for length in [0, 1, 511, 513, 4096] {
            let range = Range::new(1, 1, 100).unwrap();
            assert_eq!(card.read(range, &mut vec![0; length]), Err(Error::Bounds));
            assert_eq!(card.write(range, &vec![0; length]), Err(Error::Bounds));
        }
    }
}
