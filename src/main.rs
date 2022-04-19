#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_assignments)]

pub mod util;
pub mod front;
pub mod back;
pub mod mem;
pub mod rf;
pub mod op;

use std::fs::File;
use std::io::Read;
use std::collections::{ VecDeque, BTreeMap };
use std::sync::{ Arc, RwLock };

use crate::util::*;
use crate::front::*;
use crate::back::*;
use crate::mem::*;
use crate::rf::*;
use crate::op::*;

fn main() {
    let mut f = File::open("./code/test.bin").expect("no file");
    unsafe { f.read(&mut RAM).unwrap(); }

    let mut npc: usize = 0;
    let mut fetch_stall    = false;
    let mut decode_stall   = false;
    let mut dispatch_stall = false;

    let mut ftq: Queue<usize> = Queue::new(8);
    let mut ifu = FetchUnit;

    let mut ibq: Queue<IBQEntry> = Queue::new(20);
    let mut idu = DecodeUnit { pick_offset: 0 };

    let mut opq: Queue<OPQEntry> = Queue::new(32);
    let mut prf = PhysicalRegisterFile::new();
    let mut rob: Queue<usize> = Queue::new(224);

    let mut dec_out: [Option<DecodedInst>; 4] = [None; 4];

    while clk() < 6 {
        println!("============ cycle {} ====================", clk());

        ftq.push(npc).unwrap();

        // Fetch unit
        fetch_stall = ftq.is_empty() || ibq.is_full();
        if !fetch_stall {
            ifu.cycle(&mut ftq, &mut ibq);
        } else {
            println!("[!] fetch stall");
        }

        // Decode unit
        decode_stall = ibq.len() < 2 || opq.is_full();
        if !decode_stall {
            dec_out = idu.cycle(&mut ibq);
        } else {
            dec_out = [None; 4];
            println!("[!] decode stall");
        }

        // Convert instructions into "macro-ops"
        for e in dec_out.iter() {
            if let Some(inst) = e {
                let mop = get_macro_ops(&inst);
                let ent = OPQEntry {
                    op: mop,
                    addr: inst.addr,
                };
                opq.push(ent).unwrap();
            }
        }

        for e in opq.data.iter() { println!("{:?}", e); }

        dispatch_stall = opq.is_empty() || !prf.can_alloc() || rob.is_full();
        if !dispatch_stall {
            // The dispatch window is 6 micro-ops per cycle
            for _ in 0..6 {
                if let Ok(op) = opq.peek(0) {
                    //if !prf.can_alloc();
                } 
                // The op queue has been fully drained
                else { break; }
            }
        } else {
            println!("[!] dispatch stall");
        }

        npc += 0x20;

        step();
    }


}









