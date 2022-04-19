
pub static mut CLOCK: usize = 0;
pub fn clk() -> usize { unsafe { CLOCK } }
pub fn step() { unsafe { CLOCK += 1 } }
pub fn stepn(n: usize) { unsafe { CLOCK += n } }

pub const RAM_LEN: usize = 0x0200_0000;
pub static mut RAM: [u8; RAM_LEN] = [0; RAM_LEN];
pub fn read(addr: usize, len: usize) -> &'static [u8] { 
    assert!(addr+len < RAM_LEN);
    unsafe { &RAM[addr..addr+len] } 
}
pub fn read8(addr: usize) -> u8 { 
    assert!(addr < RAM_LEN);
    unsafe { RAM[addr] } 
}
pub fn read16(addr: usize) -> u16 { 
    assert!(addr+2 < RAM_LEN);
    unsafe { 
        let (b, _) = RAM.split_at(std::mem::size_of::<u16>());
        u16::from_le_bytes(b.try_into().unwrap()) 
    } 
}
pub fn read32(addr: usize) -> u32 { 
    assert!(addr+4 < RAM_LEN);
    unsafe { 
        let (b, _) = RAM.split_at(std::mem::size_of::<u32>());
        u32::from_le_bytes(b.try_into().unwrap()) 
    } 
}
pub fn write(addr: usize, data: &[u8]) {
    assert!(addr+data.len() < RAM_LEN);
    unsafe { RAM[addr..addr+data.len()].copy_from_slice(data) }
}
pub fn write8(addr: usize, data: u8) {
    assert!(addr < RAM_LEN);
    unsafe { RAM[addr] = data; }
}
pub fn write16(addr: usize, data: u16) {
    assert!(addr+2 < RAM_LEN);
    unsafe { 
        let src = std::slice::from_raw_parts(
            &data as *const u16 as *const u8,
            std::mem::size_of::<u16>()
        );
        RAM[addr..addr+src.len()].copy_from_slice(src);
    }
}
pub fn write32(addr: usize, data: u32) {
    assert!(addr+4 < RAM_LEN);
    unsafe { 
        let src = std::slice::from_raw_parts(
            &data as *const u32 as *const u8,
            std::mem::size_of::<u32>()
        );
        RAM[addr..addr+src.len()].copy_from_slice(src);
    }
}

pub fn cache_read(addr: usize) -> [u8; 32] {
    assert!(addr & 0x1f == 0);
    read(addr, 32).try_into().unwrap()
}


