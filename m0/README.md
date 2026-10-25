# M0 DFU protocol 2

USB EP0, 64-byte packets, 4 KiB buffer. Alternate **0 = NOR Flash**, **1 = SD Card**.
Vendor requests use device recipient: `0x40` OUT, `0xc0` IN. Integers are little-endian.

| Request | OUT | IN |
|---|---|---|
| 1 | — | 16 B: `M0US`, version:u16, configuration:u8, D0-held:u8, requests:u32, errors:u32 |
| 2 | value=target: enter maintenance, hold D0 | — |
| 3 | — | 32 B: NOR diagnostics |
| 4 | value=NOR slot, index=sector | 16 B: `M0SL`, version:u16, slot:u16, sector:u32, address:u32 |
| 5 | value/index=`4d30`/`5550`: allow M0 writes; `0`/`0`: revoke | 1 B: permission |
| 6 | — | 16 B: `M0SD`, version:u16, blocks:u16, LBA:u32, bytes:u32 |
| 7 | — | 48 B: `M0SI`, version:u16, block-size:u16, capacity-blocks:u64, CID:16 B, CSD:16 B |
| 8 | — | 32 B: SD diagnostics |
| 9 | — | 64 B: SD register snapshot |
| `0x10..0x17` | Select 1..8 SD blocks; LBA=value+(index<<16) | — |

NOR transfers are 4096 bytes; SD transfers are 512..4096 bytes.
Maintenance, selectors and permissions require idle DFU with no pending OUT.
Enter maintenance, select the destination, then DNLOAD. Zero-length DNLOAD ends
reception; the second GETSTATUS performs the write. UPLOAD reads media back.
DETACH requests software POR. Incomplete downloads never write; failed commits
block DETACH until that target is successfully written.
