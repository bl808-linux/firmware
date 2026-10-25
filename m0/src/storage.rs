//! Storage ownership and selection, independent of USB and DFU sequencing.
use crate::{board, dfu::Status, flash, sd};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Target {
    Nor = 0,
    Sd = 1,
}
impl Target {
    pub fn from(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::Nor),
            1 => Some(Self::Sd),
            _ => None,
        }
    }
}

/// Keep the host's selector and its validated erase unit together.
struct NorSelection {
    slot: u16,
    index: u16,
    sector: flash::Sector,
}

impl NorSelection {
    fn new(slot: u16, index: u16, allow_m0: bool) -> Option<Self> {
        Some(Self {
            slot,
            index,
            sector: flash::Sector::for_update(slot, index, allow_m0)?,
        })
    }
}

pub struct Storage {
    target: Target,
    selection: NorSelection,
    experimental_m0: bool,
    range: Option<sd::Range>,
    failed_targets: u8,
    nor: Option<flash::Nor>,
    card: Option<sd::Card>,
}
impl Storage {
    pub fn new() -> Self {
        Self {
            target: Target::Nor,
            selection: NorSelection::new(0, 0, false).unwrap(),
            experimental_m0: false,
            range: None,
            failed_targets: 0,
            nor: None,
            card: None,
        }
    }
    pub fn target(&self) -> Target {
        self.target
    }
    pub fn nor_selection(&self) -> (u16, u16, u32) {
        (
            self.selection.slot,
            self.selection.index,
            self.selection.sector.address(),
        )
    }
    pub fn m0_allowed(&self) -> bool {
        self.experimental_m0
    }
    pub fn sd_range(&self) -> Option<sd::Range> {
        self.range
    }
    pub fn card_info(&self) -> Option<&sd::Card> {
        self.card.as_ref()
    }
    pub fn select_target(&mut self, target: Target) {
        self.target = target;
    }
    pub fn enter(&mut self, target: Target) -> Result<(), Status> {
        board::hold_d0();
        if !board::d0_is_held() {
            return Err(Status::Firmware);
        }
        match target {
            Target::Nor => {
                if self.nor.is_none() {
                    self.nor = Some(flash::Nor::init().map_err(nor_error)?);
                }
            }
            Target::Sd => {
                if self.card.is_none() {
                    self.card = Some(sd::Card::init().map_err(sd_error)?);
                }
            }
        }
        Ok(())
    }
    pub fn select_nor(&mut self, slot: u16, index: u16) -> Result<(), ()> {
        self.selection = NorSelection::new(slot, index, self.experimental_m0).ok_or(())?;
        Ok(())
    }
    pub fn allow_m0(&mut self, enabled: bool) {
        self.experimental_m0 = enabled;
        self.selection =
            NorSelection::new(self.selection.slot, self.selection.index, enabled).unwrap();
    }
    pub fn select_sd(&mut self, lba: u32, blocks: u16) -> Result<(), Status> {
        // Invalid reselection invalidates the old write destination, too.
        self.range = None;
        let capacity = self.card.as_ref().ok_or(Status::Firmware)?.blocks;
        self.range = Some(sd::Range::new(lba, blocks, capacity).map_err(sd_error)?);
        Ok(())
    }
    pub fn length(&self) -> Option<usize> {
        match self.target {
            Target::Nor => Some(flash::SECTOR),
            Target::Sd => self.range.map(sd::Range::len),
        }
    }
    pub fn writable(&self) -> bool {
        match self.target {
            Target::Nor => self.nor.is_some() && self.selection.sector.writable(),
            Target::Sd => self.card.is_some() && self.range.is_some(),
        }
    }
    pub fn read(&mut self, data: &mut [u8]) -> Result<(), Status> {
        match self.target {
            Target::Nor => self
                .nor
                .as_mut()
                .ok_or(Status::Firmware)?
                .read_selected(self.selection.sector, data)
                .map_err(nor_error),
            Target::Sd => {
                let result = self
                    .card
                    .as_mut()
                    .ok_or(Status::Firmware)?
                    .read(self.range.ok_or(Status::Address)?, data)
                    .map_err(sd_error);
                if result.is_err_and(|status| status != Status::Address) {
                    self.card = None;
                    self.range = None;
                }
                result
            }
        }
    }
    pub fn failed(&self) -> bool {
        self.failed_targets != 0
    }
    pub fn write(&mut self, data: &[u8]) -> Result<(), Status> {
        let failure_bit = 1 << self.target as u8;
        self.failed_targets |= failure_bit;
        if self.length() != Some(data.len()) {
            return Err(Status::Address);
        }
        match self.target {
            Target::Nor => self
                .nor
                .as_mut()
                .ok_or(Status::Firmware)?
                .replace_sector(self.selection.sector, data)
                .map_err(nor_error)?,
            Target::Sd => {
                let result = self
                    .card
                    .as_mut()
                    .ok_or(Status::Firmware)?
                    .write(self.range.ok_or(Status::Address)?, data)
                    .map_err(sd_error);
                if result.is_err_and(|status| status != Status::Address) {
                    self.card = None;
                    self.range = None;
                }
                result?;
            }
        }
        self.failed_targets &= !failure_bit;
        Ok(())
    }
    pub fn prepare_reset(&mut self) -> Result<(), Status> {
        if self.failed() {
            return Err(Status::NotDone);
        }
        if let Some(nor) = self.nor.as_mut() {
            nor.prepare_reset().map_err(nor_error)?;
        }
        if let Some(card) = self.card.as_mut() {
            card.prepare_reset().map_err(sd_error)?;
        }
        Ok(())
    }
}
fn nor_error(error: flash::Error) -> Status {
    flash::record_error(error);
    match error {
        flash::Error::Erase => Status::Erase,
        flash::Error::Verify => Status::Verify,
        flash::Error::Bounds => Status::Address,
        _ => Status::Write,
    }
}
fn sd_error(error: sd::Error) -> Status {
    sd::record_error(error);
    match error {
        sd::Error::Bounds => Status::Address,
        sd::Error::Verify => Status::Verify,
        _ => Status::Write,
    }
}
