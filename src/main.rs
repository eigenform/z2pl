#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_assignments)]

pub mod util;
pub mod front;

pub mod dispatch;
pub mod issue;
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
use crate::issue::*;
use crate::retire::*;
use crate::mem::*;
use crate::rf::*;
use crate::op::*;
use crate::exec::*;

fn main() {

    let mut f = File::open("./code/test.bin").expect("no file");
    unsafe { f.read(&mut RAM).unwrap(); }

    // Branch prediction
    let mut bpu = BranchPredictionUnit::new();
    let mut btb = BranchTargetBuffer::new();

    // Next PC
    let mut pq: Queue<usize> = Queue::new(32);
    let mut npc = NextPCLogic;
    let mut next_pc: usize = 0;

    // Instruction fetch
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

    // Out-of-order issue
    let mut issue_stall = false;
    let mut isu = IssueUnit;
    let mut alu_sched = [ALUScheduler::new(); 4];
    let mut agu_sched = AGUScheduler::new();

    // Execution units
    let mut prf = PhysicalRegisterFile::new();
    let mut eu  = ExecutionUnits::new();

    // Retire control unit
    let mut rat = RegisterAliasTable::new();
    let mut rob = ReorderBuffer::new(224);
    let mut rcu = RetireControlUnit;

    while clk() < 32 {
        println!("============ cycle {} ====================", clk());

        //// Branch prediction unit
        //bpu.cycle(&mut btb, &mut pq);
        //// Next program counter
        //npc.cycle(&mut next_pc, &mut pq, &mut ftq);
        //// Instruction fetch unit 
        //ifu.cycle(&mut ftq, &mut ibq);
        //// Instruction decode
        //idu.cycle(&mut ibq, &mut opq, &mut bpu);
        //// Instruction dispatch
        //dispatch.cycle(
        //    &mut btb, &mut opq, 
        //    &mut alu_sched, &mut agu_sched, 
        //    &mut prf, &mut rob, &mut rat
        //);
        //// Execution units
        //eu.cycle(&mut rob, &mut prf);
        //// Instruction issue
        //isu.cycle(&mut alu_sched, &mut eu);
        //// Retire control unit
        //rcu.cycle(&mut rob, &mut rat);
        //rat.print(&prf);

        rcu.cycle(&mut rob, &mut rat);
        rat.print(&prf);
        eu.cycle(&mut rob, &mut prf);
        isu.cycle(&mut alu_sched, &mut eu);
        dispatch.cycle(
            &mut btb, &mut opq, 
            &mut alu_sched, &mut agu_sched, 
            &mut prf, &mut rob, &mut rat
        );
        idu.cycle(&mut ibq, &mut opq, &mut bpu);
        ifu.cycle(&mut ftq, &mut ibq);
        npc.cycle(&mut next_pc, &mut pq, &mut ftq);
        bpu.cycle(&mut btb, &mut pq);


        step();
    }

}



