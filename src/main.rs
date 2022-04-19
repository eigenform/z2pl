#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_assignments)]


pub mod util;
pub mod front;

pub mod dispatch;
pub mod schedule;
pub mod retire;

pub mod mem;
pub mod rf;
pub mod op;

use std::fs::File;
use std::io::Read;
use std::collections::{ VecDeque, BTreeMap };
use std::sync::{ Arc, RwLock };

use crate::util::*;
use crate::front::*;
use crate::dispatch::*;
use crate::schedule::*;
use crate::retire::*;
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

        println!("FTQ push {:08x}", npc);
        ftq.push(npc).unwrap();

        // Instruction fetch
        fetch_stall = ftq.is_empty() || ibq.is_full();
        if !fetch_stall {
            ifu.cycle(&mut ftq, &mut ibq);
        } else {
            println!("[!] fetch stall");
        }

        // Instruction decode
        decode_stall = ibq.len() < 2 || opq.is_full();
        if !decode_stall {
            dec_out = idu.cycle(&mut ibq);
        } else {
            dec_out = [None; 4];
            println!("[!] decode stall");
        }

        // Convert instructions into "macro-ops" and add to the op queue
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

        for e in opq.data.iter() { println!("{:x?}", e); }

        // The dispatch window is 6 macro-ops per cycle
        'dispatch: for idx in 0..6 {
            println!("Dispatch window slot {}", idx);

            if rob.is_full() {
                println!("[!] stalled for ROB allocation");
                break 'dispatch;
            }

            if let Ok(e) = opq.peek(0) {

                // Find a free physical register if necessary
                let dst = if let Some(rd) = e.op.reg_result() {
                    if let Some(prn) = prf.find() {
                        Some((rd, prn))
                    } else {
                        println!("[!] stalled for PRF allocation");
                        break 'dispatch;
                    }
                } else { None };

                let robent = ROBEntry {
                    addr: e.addr,
                    mop: e.op, uop: Uop::Nop,
                    sched: SchedulerId::None,
                    dst, complete: false
                };
                println!("{:?}", robent);

            } else {
                println!("[!] Op queue exhausted");
                break 'dispatch;
            }
        }

        npc += 0x20;

        step();
    }


}









