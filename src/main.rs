
pub mod util;
pub mod fetch;

use std::fs::File;
use std::io::Read;
use std::collections::{ VecDeque, BTreeMap };
use std::sync::{ Arc, RwLock };
use crate::util::*;
use crate::fetch::*;

use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
};

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


#[derive(Copy, Clone)]
pub struct CacheLine {
    pub addr: usize,
    pub data: [u8; 64]
}

#[derive(Copy, Clone)]
pub struct HalfLine {
    pub addr: usize,
    pub data: [u8; 32]
}

#[derive(Copy, Clone)]
pub struct IBQEntry {
    pub addr: usize,
    pub data: [u8; 16]
}

pub struct DecodedInst {
    pub inst: Instruction,
    pub bytes: Vec<u8>,
}

fn main() {
    let mut f = File::open("./code/test.bin").expect("no file");
    unsafe { f.read(&mut RAM).unwrap(); }

    let mut pc: usize = 0;
    let mut ftq: Queue<usize> = Queue::new(8);
    let mut ibq: Queue<IBQEntry> = Queue::new(20);
    let mut fetch_stall = false;
    let mut decode_stall = false;

    let mut pick_off = 0;

    while clk() < 4 {
        println!("============ cycle {} ====================", clk());
        ftq.push(pc).unwrap();

        fetch_stall = ftq.is_empty() || ibq.is_full();

        if !fetch_stall {
            // Get a half-line (32B) from the L1 cache per-cycle
            let addr = ftq.pop().unwrap();
            println!("FTQ pop {:08x}", addr);
            let data = cache_read(addr);
            println!("L1 cache read {:08x}", addr);
            //let line = HalfLine { addr, data };

            // Push the next half-line onto the fetch target queue
            //ftq.push(addr + 0x20).unwrap();
            //println!("FTQ push {:08x}", addr + 0x20);

            // Push onto the instruction byte queue
            ibq.push(IBQEntry { 
                addr: addr + 0x00, data: data[0x00..0x10].try_into().unwrap() 
            });
            println!("IBQ push {:08x}", addr);
            ibq.push(IBQEntry { 
                addr: addr + 0x10, data: data[0x10..].try_into().unwrap() 
            });
            println!("IBQ push {:08x}", addr + 0x10);
        } else {
            println!("[!] fetch stall");
        }

        decode_stall = ibq.len() < 2;

        if !decode_stall {
            // Use the oldest two entries for the pick window
            let bot = ibq.peek(0).unwrap();
            let top = ibq.peek(1).unwrap();
            let mut pick = [0u8; 32];
            pick[0x00..0x10].copy_from_slice(&bot.data);
            pick[0x10..].copy_from_slice(&top.data);
            println!("Pick window (pick_off={:02x}):", pick_off);
            println!(" {:08x}: {:02x?}", bot.addr, bot.data);
            println!(" {:08x}: {:02x?}", top.addr, top.data);

            let mut inst = Instruction::default();
            let mut decoder = Decoder::with_ip(
                64, &pick[pick_off..], 0, DecoderOptions::NONE
            );
            let mut slots = Vec::new();
            let mut pick_cursor = pick_off;

            for idx in 0..4 {
                decoder.decode_out(&mut inst);
                if idx != 0 && inst.len() > 8 {
                    break;
                }
                if inst.is_invalid() {
                    break;
                }
                let bytes = &pick[pick_cursor..pick_cursor + inst.len()];
                pick_cursor += inst.len();
                slots.push((inst, bytes));
            }

            for (i, s) in slots.iter().enumerate() {
                println!("slot {}: pick_cursor={:02x} {:?} {:02x?}", 
                         i, s.0.ip(), s.0.code(), s.1);
            }
            println!("stopped at pick_cursor={:02x}", pick_cursor);

            match pick_cursor {
                0x00..=0x0f => {
                    pick_off = pick_cursor;
                },
                0x10..=0x1f => {
                    pick_off = pick_cursor - 0x10;
                    ibq.pop();
                },
                _ => unreachable!(),
            }
        } else {
            println!("[!] decode stall");
        }


        pc += 0x20;

        step();
    }


}









