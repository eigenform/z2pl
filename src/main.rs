#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_assignments)]


pub mod util;
pub mod front;

pub mod dispatch;
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
    let mut dec_out: [Option<DecodedInst>; 4] = [None; 4];

    let mut opq: Queue<OPQEntry> = Queue::new(32);
    let mut dis = DispatchUnit::new();

    let mut alu_sched = [ALUScheduler::new(); 4];
    let mut agu_sched = AGUScheduler::new();

    let mut rat = RegisterAliasTable::new();
    let mut prf = PhysicalRegisterFile::new();
    let mut rob = ReorderBuffer::new();


    while clk() < 6 {
        println!("============ cycle {} ====================", clk());

        // Push some predicted fetch target address onto the FTQ.
        // When no prediction occurs, this is the next-sequential fetch
        // block address.

        println!("[FTQ] Push {:08x} onto the FTQ", npc);
        ftq.push(npc).unwrap();

        // Consume an entry from the FTQ and read 32 bytes from the L1 cache
        // per cycle. Push the resulting bytes onto the IBQ.

        fetch_stall = ftq.is_empty() || ibq.is_full();
        if !fetch_stall {
            ifu.cycle(&mut ftq, &mut ibq);
        } else {
            println!("[IFU] Fetch stall");
        }

        // Scan the oldest IBQ entries (a 32-byte pick window), decoding up
        // to four instructions.

        decode_stall = ibq.len() < 2 || opq.is_full();
        if !decode_stall {
            dec_out = idu.cycle(&mut ibq);
        } else {
            dec_out = [None; 4];
            println!("[IDU] Decode stall");
        }

        // Convert decoded instructions into "macro-ops" and add them to
        // the macro-op queue.

        for e in dec_out.iter() {
            if let Some(inst) = e {
                let mop = get_macro_ops(&inst);
                let ent = OPQEntry {
                    op: mop,
                    addr: inst.addr,
                };
                opq.push(ent).unwrap();
                //println!("[IDU] Pushed {:x?}", ent);
            }
        }

        // Dispatch up to 6 macro-ops per cycle from the op queue.
        //
        // NOTE: I think dispatch is technically out-of-order, although right
        // now, this is technically preserving the program order.

        'dispatch: for idx in 0..6 {
            // Get a copy of the next candidate for dispatch
            let (mop_addr, mop) = if let Ok(e) = opq.peek(0) { (e.addr, e.op) } 
            else { 
                println!("[DIS] Stalled for empty op queue");
                break 'dispatch;
            };

            // Decompose a macro-op into one or two micro-ops
            let mut uops = Uop::from_mop(mop, mop_addr);
            println!("[DIS] Decomposed macro-op {:x?}", mop);
            for u in uops.iter() {
                println!("[DIS]    {:?}", u.kind);
            }

            // Get the number of required physical registers
            let num_prn_alloc = uops.iter().map(|u| u.preg_allocs()).sum();
            let num_prn_free = prf.free_regs();
            println!("[PRA] PRF has {} free physical registers, need {}", 
                     num_prn_free, num_prn_alloc);

            // Get the number of required scheduler entries
            let num_alu_alloc = uops.iter().filter(|&u| u.is_alu()).count();
            let num_agu_alloc = uops.iter().filter(|&u| u.is_agu()).count();
            let num_alu_free: usize = alu_sched.iter()
                .map(|s| s.num_free()).sum();
            let num_agu_free = agu_sched.num_free();
            println!("[SCH] ALU schedulers have {} total free slots, need {}", 
                     num_alu_free, num_alu_alloc);
            println!("[SCH] AGU scheduler has {} free slots, need {}", 
                     num_agu_free, num_agu_alloc);

            // Get the number of required ROB entries
            let num_rob_alloc = uops.len();
            let num_rob_free  = rob.num_free();
            println!("[SCH] ROB has {} free slots, need {}",
                     num_rob_free, num_rob_alloc);

            // Determine if all resources are available for allocation.
            // If we don't have the resources, stall dispatch
            let prn_alloc_ok = num_prn_free >= num_prn_alloc;
            let alu_alloc_ok = num_alu_free >= num_alu_alloc;
            let agu_alloc_ok = num_agu_free >= num_agu_alloc;
            let rob_alloc_ok = num_rob_free >= num_rob_alloc;
            if !prn_alloc_ok {
                println!("[PRA] Stalled for physical register allocation");
                break 'dispatch;
            }
            if !alu_alloc_ok {
                println!("[SCH] Stalled for ALU scheduler allocation");
                break 'dispatch;
            }
            if !agu_alloc_ok {
                println!("[SCH] Stalled for AGU scheduler allocation");
                break 'dispatch;
            }
            if !rob_alloc_ok {
                println!("[SCH] Stalled for ROB allocation");
                break 'dispatch;
            }

            for uop in uops.iter_mut() {
                // Allocate architectural destination register.
                for eff in uop.eff.iter_mut() {
                    if let Effect::RegWrite(rd, prn) = eff {
                        if prn == &Prn::alloc() {
                            let nprn = prf.alloc().unwrap();
                            println!("[PRA] Allocated {:?} for {:?}", nprn, rd);
                            rat.bind(rd.clone(), nprn);
                            *eff = Effect::RegWrite(rd.clone(), nprn);
                        }
                    }
                }

                // Rename architectural source registers
                for arg in uop.arg.iter_mut() {
                    if let Storage::Arn(r) = arg {
                        let p = rat.resolve(r);
                        println!("[RAT] Renamed {:?} to {:?}", r, p);
                        *arg = Storage::Prn(p);
                    }
                }

                // Send this micro-op to a scheduler.
                // NOTE: This doesn't make any "real" attempt to actually 
                // balance the ALU scheduling.
                match uop.kind {
                    UopKind::Alu(_) => {
                        let r = ALUReservation { mop, uop: *uop };

                        // Naively prioritize the emptiest queue
                        let (i, mut tgt) = alu_sched.iter_mut()
                            .enumerate().max_by(|(i,x),(j,y)| { 
                                x.num_free().cmp(&y.num_free()) 
                        }).unwrap();

                        tgt.alloc(r).unwrap();
                        println!("[DIS] Sent to ALSQ{}", i);
                    },
                    UopKind::Agu(_) => {
                        let r = AGUReservation { mop, uop: *uop };
                        agu_sched.alloc(r).unwrap();
                        println!("[DIS] Sent to AGSQ");
                    },
                    _ => unreachable!(),
                }

                // Allocate a reorder buffer entry for this micro-op
                let rob_ent = ROBEntry::new(mop, *uop);
                rob.push(rob_ent).unwrap();

            }
            // Pop this macro-op from the queue
            opq.pop().unwrap();


        }

        npc += 0x20;

        step();
    }


}









