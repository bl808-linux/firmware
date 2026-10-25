//! DFU 1.1 transfers into a bounded SRAM scratch buffer. Never executes data.

pub const CAPACITY: usize = 4096;
pub const PACKET: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Idle = 2,
    DownloadSync = 3,
    DownloadIdle = 5,
    ManifestSync = 6,
    Manifest = 7,
    UploadIdle = 9,
    Error = 10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Ok = 0,
    Write = 3,
    Erase = 4,
    Verify = 7,
    Address = 8,
    NotDone = 9,
    Firmware = 10,
    Unknown = 14,
    StalledPacket = 15,
}

// One USB polling context owns this SRAM, separate from the 4 KiB CPU stack.
#[unsafe(no_mangle)]
static mut DFU_DATA: [u8; CAPACITY] = [0; CAPACITY];

pub struct Transfer {
    state: State,
    status: Status,
    next: u16,
    length: usize,
    complete: bool,
    upload_offset: usize,
}

impl Transfer {
    pub const fn new() -> Self {
        Self {
            state: State::Idle,
            status: Status::Ok,
            next: 0,
            length: 0,
            complete: false,
            upload_offset: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn is_idle(&self) -> bool {
        self.state == State::Idle
    }

    pub fn can_select_alternate(&self) -> bool {
        matches!(
            self.state,
            State::Idle
                | State::DownloadSync
                | State::DownloadIdle
                | State::UploadIdle
                | State::Error
        )
    }

    pub fn can_detach(&self) -> bool {
        matches!(self.state, State::Idle | State::UploadIdle)
    }

    fn fail<T>(&mut self, status: Status) -> Result<T, ()> {
        self.status = status;
        self.state = State::Error;
        self.complete = false;
        Err(())
    }

    /// Validate before accepting an OUT packet. Zero length ends a download.
    pub fn download(&mut self, block: u16, length: usize) -> Result<(), ()> {
        if self.state != State::Idle && self.state != State::DownloadIdle {
            return self.fail(Status::StalledPacket); // errSTALLEDPKT
        }
        if self.state == State::Idle {
            self.next = 0;
            self.length = 0;
            self.complete = false;
        }
        if block != self.next || length > PACKET || self.length + length > CAPACITY {
            return self.fail(Status::Address); // errADDRESS
        }
        if length == 0 {
            if self.state != State::DownloadIdle || self.length == 0 {
                return self.fail(Status::NotDone); // errNOTDONE
            }
            self.state = State::ManifestSync;
        } else {
            self.state = State::DownloadSync;
        }
        Ok(())
    }

    pub fn accept(&mut self, data: &[u8]) {
        // Only called after a validated packet has been received in full.
        assert!(self.state == State::DownloadSync && data.len() <= PACKET);
        assert!(self.length + data.len() <= CAPACITY);
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                core::ptr::addr_of_mut!(DFU_DATA)
                    .cast::<u8>()
                    .add(self.length),
                data.len(),
            );
        }
        self.length += data.len();
        self.next += 1;
    }

    pub fn get_status(&mut self) -> [u8; 6] {
        self.state = match self.state {
            State::DownloadSync => State::DownloadIdle,
            State::ManifestSync => State::Manifest,
            State::Manifest => {
                self.complete = true;
                State::Idle
            }
            state => state,
        };
        [
            self.status as u8,
            if self.state == State::Manifest {
                100
            } else {
                0
            },
            0,
            0,
            self.state as u8,
            0,
        ]
    }

    pub fn upload(&mut self, block: u16, requested: usize) -> Result<([u8; PACKET], usize), ()> {
        if !self.complete
            || requested == 0
            || requested > PACKET
            || (self.state != State::Idle && self.state != State::UploadIdle)
        {
            return self.fail(Status::NotDone);
        }
        if self.state == State::Idle {
            self.next = 0;
            self.upload_offset = 0;
        }
        if block != self.next {
            return self.fail(Status::Address);
        }
        let count = requested.min(self.length - self.upload_offset);
        let mut data = [0; PACKET];
        unsafe {
            core::ptr::copy_nonoverlapping(
                core::ptr::addr_of!(DFU_DATA)
                    .cast::<u8>()
                    .add(self.upload_offset),
                data.as_mut_ptr(),
                count,
            );
        }
        self.upload_offset += count;
        self.next += 1;
        self.state = if count < requested {
            State::Idle
        } else {
            State::UploadIdle
        };
        Ok((data, count))
    }

    pub fn abort(&mut self) -> Result<(), ()> {
        match self.state {
            State::DownloadSync | State::DownloadIdle => *self = Self::new(),
            State::Idle | State::UploadIdle => {
                self.state = State::Idle;
                self.status = Status::Ok;
            }
            _ => return self.fail(Status::StalledPacket),
        }
        Ok(())
    }

    pub fn clear_error(&mut self) -> Result<(), ()> {
        if self.state != State::Error {
            return Err(());
        }
        *self = Self::new();
        Ok(())
    }

    pub fn transport_error(&mut self) {
        let _ = self.fail::<()>(Status::Firmware); // errFIRMWARE: incomplete transport, no commit
    }

    pub fn storage_error(&mut self, status: Status) {
        let _ = self.fail::<()>(status);
    }

    pub fn select_alternate(&mut self, previous: u8, selected: u8) {
        // Host libraries reselect/release alternate zero between CLI calls.
        // Selecting the same target must preserve a completed SRAM download.
        if previous != selected {
            *self = Self::new();
        }
    }

    pub fn manifest_data(&self) -> Option<&[u8]> {
        if self.state != State::Manifest {
            return None;
        }
        // The single polling context cannot receive another packet while borrowed.
        unsafe {
            Some(core::slice::from_raw_parts(
                core::ptr::addr_of!(DFU_DATA).cast::<u8>(),
                self.length,
            ))
        }
    }

    pub fn load_readback(&mut self, length: usize, read: impl FnOnce(&mut [u8]) -> bool) -> bool {
        if self.state != State::Idle || length == 0 || length > CAPACITY {
            return false;
        }
        let data = unsafe {
            core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(DFU_DATA).cast::<u8>(), length)
        };
        if !read(data) {
            self.storage_error(Status::Unknown);
            return false;
        }
        self.length = length;
        self.complete = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_capacity_round_trip_and_eof() {
        let mut t = Transfer::new();
        for block in 0..64 {
            t.download(block, PACKET).unwrap();
            t.accept(&[block as u8; PACKET]);
            assert_eq!(t.get_status()[4], State::DownloadIdle as u8);
        }
        t.download(64, 0).unwrap();
        assert_eq!(t.get_status()[4], State::Manifest as u8);
        assert_eq!(t.get_status()[4], State::Idle as u8);
        for block in 0..64 {
            let (data, n) = t.upload(block, PACKET).unwrap();
            assert_eq!(n, PACKET);
            assert_eq!(data, [block as u8; PACKET]);
        }
        assert_eq!(t.upload(64, PACKET).unwrap().1, 0);
        assert_eq!(t.state, State::Idle);
    }

    #[test]
    fn overflow_and_bad_sequence_invalidate_data() {
        let mut t = Transfer::new();
        for block in 0..64 {
            t.download(block, PACKET).unwrap();
            t.accept(&[1; PACKET]);
            t.get_status();
        }
        assert!(t.download(64, 1).is_err());
        assert_eq!(t.get_status()[0], 8);
        t.clear_error().unwrap();
        assert!(t.upload(0, PACKET).is_err());
        t.clear_error().unwrap();
        assert!(t.download(1, 1).is_err());
    }

    #[test]
    fn abort_and_transport_failure_do_not_publish_partial_data() {
        let mut t = Transfer::new();
        t.download(0, 3).unwrap();
        t.accept(b"abc");
        t.get_status();
        t.abort().unwrap();
        assert!(t.upload(0, PACKET).is_err());
        t.clear_error().unwrap();
        t.download(0, PACKET).unwrap();
        t.transport_error();
        assert_eq!(t.state, State::Error);
        t.clear_error().unwrap();
        assert!(t.download(0, 0).is_err());
    }

    #[test]
    fn failed_storage_read_cannot_publish_partial_readback() {
        let mut t = Transfer::new();
        assert!(!t.load_readback(CAPACITY, |data| {
            data[..3].copy_from_slice(b"bad");
            false
        }));
        assert_eq!(t.get_status()[0], 0x0e);
        assert!(t.upload(0, PACKET).is_err());
        t.clear_error().unwrap();
        assert!(t.load_readback(CAPACITY, |data| {
            data.fill(0xa5);
            true
        }));
        assert_eq!(t.upload(0, PACKET).unwrap(), ([0xa5; PACKET], PACKET));
    }

    #[test]
    fn same_alternate_preserves_data_between_host_connections() {
        let mut t = Transfer::new();
        t.download(0, 3).unwrap();
        t.accept(b"abc");
        t.get_status();
        t.download(1, 0).unwrap();
        t.get_status();
        t.get_status();
        t.select_alternate(0, 0);
        let (data, length) = t.upload(0, PACKET).unwrap();
        assert_eq!(&data[..length], b"abc");
        t.select_alternate(0, 1);
        assert!(t.upload(0, PACKET).is_err());
    }
    #[test]
    fn shorter_readback_never_uploads_a_previous_buffer_tail() {
        let mut t = Transfer::new();
        assert!(t.load_readback(CAPACITY, |data| {
            data.fill(0xa5);
            true
        }));
        t.abort().unwrap();
        assert!(!t.load_readback(CAPACITY + 1, |_| panic!("oversize read")));
        assert!(t.load_readback(512, |data| {
            assert_eq!(data.len(), 512);
            data.fill(0x5a);
            true
        }));
        for block in 0..8 {
            assert_eq!(t.upload(block, PACKET).unwrap(), ([0x5a; PACKET], PACKET));
        }
        assert_eq!(t.upload(8, PACKET).unwrap().1, 0);
    }
}
