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
pub mod exec;
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
use crate::exec::*;

fn main() {
    let mut f = File::open("./code/test.bin").expect("no file");
    unsafe { f.read(&mut RAM).unwrap(); }

    let mut npc: usize     = 0;


    // Instruction fetch
    let mut pred_stall  = false;
    let mut fetch_stall = false;
    let mut ftq: Queue<usize> = Queue::new(8);
    let mut ifu = FetchUnit;

    // Instruction decode
    let mut decode_stall = false;
    let mut ibq: Queue<IBQEntry> = Queue::new(20);
    let mut idu = DecodeUnit { pick_offset: 0 };
    let mut dec_out: [Option<DecodedInst>; 4] = [None; 4];

    // In-order dispatch
    let mut dispatch_stall = false;
    let mut opq: Queue<OPQEntry> = Queue::new(32);
    let mut dispatch = DispatchUnit;

    // Dynamic scheduling/out-of-order execution
    let mut issue_stall = false;
    let mut alu_sched = [ALUScheduler::new(); 4];
    let mut agu_sched = AGUScheduler::new();
    let mut rob = ReorderBuffer::new(224);
    let mut rat = RegisterAliasTable::new();
    let mut prf = PhysicalRegisterFile::new();
    let mut alu = [ALU::new(); 4];


    while clk() < 32 {
        println!("============ cycle {} ====================", clk());

        // Push some predicted fetch target address onto the FTQ.
        // When no prediction occurs, this is the next-sequential fetch
        // block address.

        pred_stall = ftq.is_full();
        if !pred_stall {
            println!("[FTQ] Push {:08x} onto the FTQ", npc);
            ftq.push(npc).unwrap();
        } else {
            println!("[FTQ] NPC stall");
        }

        // Consume an entry from the FTQ and read 32 bytes from the L1 cache
        // per cycle. Push the resulting bytes onto the IBQ.
        //
        // NOTE: Right now we assume that the fetch unit *always* pushes
        // two entries onto the IBQ (otherwise, if there aren't two free
        // entries in the IBQ, we stall the fetch unit).

        fetch_stall = ftq.is_empty() || ibq.num_free() < 2;
        if !fetch_stall {
            ifu.cycle(&mut ftq, &mut ibq);
        } else {
            println!("[IFU] Fetch stall");
        }

        // Scan the oldest IBQ entries (a 32-byte pick window), decoding up
        // to four instructions.
        //
        // Convert decoded instructions into macro-ops and push them
        // onto the micro-op queue
 
        decode_stall = ibq.len() < 2 || opq.is_full();
        dec_out = if !decode_stall {
            idu.cycle(&mut ibq)
        } else {
            println!("[IDU] Decode stall");
            [None; 4]
        };

        for inst in dec_out.iter().filter_map(|i| *i) {
            opq.push(OPQEntry { 
                op: get_macro_ops(&inst), 
                addr: inst.addr 
            }).unwrap();
        }

        // Dispatch up to 6 macro-ops per cycle from the op queue.
        // For each macro-op, this entails (not necessarily in this order):
        //
        // - Converting into one or more micro-ops
        // - Renaming operands into physical registers
        // - Allocating physical registers for results
        // - Allocating a reorder buffer entry
        // - Allocating a scheduler entry
        //
        // After this point, micro-ops are issued, executed, and completed
        // out-of-order.

        dispatch.cycle(
            &mut opq, 
            &mut alu_sched, &mut agu_sched, 
            &mut prf, &mut rob, &mut rat
        );
        println!("      --------------------------------------");

        // Iterate over all busy ALUs and update the status of any in-flight
        // operations.

        for (idx, tgt_alu) in alu.iter_mut().enumerate() {
            println!("[ALU] ALU{} status:", idx);
            let res = tgt_alu.cycle(&mut rob, &mut prf);
            match res {
                Ok(comp) => {
                    println!("[ALU]   {:08x}: {:?}, rob_idx={} complete", 
                        comp.uop.addr, comp.uop.kind, comp.rob_idx);
                    rob.get_mut(comp.rob_idx).unwrap().complete = true;
                },
                Err(ALUErr::Empty) => {
                    println!("[ALU]   Empty");
                },
                Err(ALUErr::PendingCompletion) => {
                    let op = tgt_alu.op.unwrap();
                    println!("[ALU]   {:08x}: {:?}", op.uop.addr, op.uop.kind);
                },
            }
        }

        println!("      --------------------------------------");



        // Iterate over all ALU schedulers and attempt to fire any pending
        // reservations that are ready-for-issue.
        //
        // Each ALQ can only issue 1 micro-op per cycle.

        let mut free_alus = alu.iter_mut().enumerate()
            .filter(|(idx, s)| s.op.is_none());
        for (idx, alq) in alu_sched.iter_mut().enumerate() {
            println!("[ISS] Checking ALQ{}", idx);
            println!("[ISS]   {} pending reservation[s]", alq.num_pending());

            // Try to get a mutable reference to an unoccupied ALU.
            // Otherwise, if all ALUs are currently busy, no more micro-ops
            // can be issued during this cycle.
            if let Some((alu_idx, tgt_alu)) = free_alus.next() {

                // Try to find a reservation which is ready-for-issue.
                //
                // If there are none, move on to the next ALQ.
                // Otherwise, *consume* the reservation from the ALQ and
                // pass it onto the appropriate ALU.
                match alq.take_ready() {
                    None => {
                        println!("[ISS]   No ready-to-issue reservations");
                        continue;
                    },
                    Some(iss_res) => {
                        println!("[ISS]   ALU{} issued {:08x}: {:?}", 
                                 alu_idx, iss_res.uop.addr, iss_res.uop.kind);
                        tgt_alu.do_issue(clk(), iss_res);
                    },
                }
            } else {
                println!("[ISS]   No free ALUs to consume a reservation");
                println!("[ISS]   Issue stalled for ALU availability");
                break;
            }
        }
        println!("      --------------------------------------");

        // Retire and commit up to 8 micro-ops per cycle.

        println!("[RCU] Reorder buffer status:");
        println!("[RCU]   In-flight:    {}", rob.num_used());
        println!("[RCU]   Free entries: {}", rob.num_free());
        println!("[RCU]   Retire ptr:   {}", rob.retire_ptr);
        println!("[RCU]   Dispatch ptr: {}", rob.dispatch_ptr);

        for i in 0..8 {
            match rob.pop() {
                Ok((idx, ent)) => {
                    if ent.uop.eff != [Effect::None, Effect::None] {
                        unimplemented!("effects unimplemented");
                    }
                    println!("[RCU] Retired entry {} ({}/8): {:08x} {:?}",
                             idx, i, ent.uop.addr, ent.uop.kind);
                },
                Err(ROBErr::Incomplete) => {
                    let front = rob.get_front().unwrap();
                    println!("[RCU] Commit stalled for {:08x} {:?}",
                             front.uop.addr, front.uop.kind);
                    break;
                }
                Err(ROBErr::Empty) => {
                    println!("[RCU] Reorder buffer is empty");
                    break;
                },
                Err(e) => unreachable!("{:?}", e),
            }
        }

        npc += 0x20;
        step();
    }

}



