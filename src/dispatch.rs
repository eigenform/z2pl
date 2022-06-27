
use crate::front::*;
use crate::rf::*;
use crate::op::*;
use crate::retire::*;
use crate::issue::*;
use crate::util::*;

/// An entry in the macro-op queue.
#[derive(Debug, Copy, Clone)]
pub struct OPQEntry {
    /// The program counter value associated with this instruction
    pub addr: usize,
    pub op: MacroOp,
}

#[derive(Debug)]
pub enum DispatchErr {
    /// The macro-op queue was empty
    OPQEmpty,
    /// Could not allocate a reorder buffer entry.
    ROBAlloc, 
    /// Could not allocate a physical register.
    PRFAlloc, 
    /// Could not reserve an ALU scheduler queue entry.
    ALQAlloc, 
    /// Could not reserve an AGU scheduler queue entry.
    AGQAlloc,
}

/// Abstract representation of the dispatch unit.
pub struct DispatchUnit;
impl DispatchUnit {

    /// Dispatch up to 6 macro-ops per cycle from the op queue.
    /// For each macro-op, this entails (not necessarily in this order):
    ///
    /// - Converting into one or more micro-ops
    /// - Renaming operands into physical registers
    /// - Allocating physical registers for results
    /// - Allocating a reorder buffer entry
    /// - Allocating a scheduler entry
    ///
    /// After this point, micro-ops are issued, executed, and completed
    /// out-of-order.

    pub fn cycle(&mut self, 
        btb: &mut BranchTargetBuffer,
        opq: &mut Queue<OPQEntry>,
        alu_sched: &mut [ALUScheduler; 4],
        agu_sched: &mut AGUScheduler,
        prf: &mut PhysicalRegisterFile, 
        rob: &mut ReorderBuffer,
        rat: &mut RegisterAliasTable,
    ) {
        'dispatch: for idx in 0..6 {

            // Get a reference to the next candidate for dispatch.
            let (mop_addr, mop) = if let Ok(e) = opq.peek(0) { (e.addr, e.op) } 
            else { 
                println!("[SCH] Op queue is empty, nothing to dispatch");
                break 'dispatch;
            };

            // Decompose a macro-op into one or two micro-ops
            let mut uops = Uop::from_mop(mop, mop_addr);
            println!("[SCH] Trying to dispatch macro-op #{} {:x?}", idx, mop);

            // Get the number of required physical registers
            let num_prn_alloc = uops.iter().map(|u| u.preg_allocs()).sum();
            let num_prn_free = prf.free_regs();

            // Get the number of required scheduler entries
            let num_alu_alloc = uops.iter().filter(|&u| u.is_alu()).count();
            let num_agu_alloc = uops.iter().filter(|&u| u.is_agu()).count();
            let num_alu_free: usize = alu_sched.iter()
                .map(|s| s.num_free()).sum();
            let num_agu_free = agu_sched.num_free();

            // Get the number of required ROB entries
            let num_rob_alloc = uops.len();
            let num_rob_free  = rob.num_free();

            // Determine if all resources are available for allocation.
            // If we don't have the resources, stall dispatch
            let prn_alloc_ok = num_prn_free >= num_prn_alloc;
            let alu_alloc_ok = num_alu_free >= num_alu_alloc;
            let agu_alloc_ok = num_agu_free >= num_agu_alloc;
            let rob_alloc_ok = num_rob_free >= num_rob_alloc;
            if !rob_alloc_ok {
                println!("[SCH] Stalled for ROB allocation");
                println!("[SCH] Free ROB slots:   {:3} (need {})",
                         num_rob_free, num_rob_alloc);
                break 'dispatch;
            }
            if !prn_alloc_ok {
                println!("[SCH] Stalled for physical register allocation");
                println!("[SCH] Free PRF entries: {:3} (need {})", 
                         num_prn_free, num_prn_alloc);
                break 'dispatch;
            }
            if !alu_alloc_ok {
                println!("[SCH] Stalled for ALU scheduler allocation");
                println!("[SCH] Free ALSQ slots:  {:3} (need {})", 
                         num_alu_free, num_alu_alloc);
                break 'dispatch;
            }
            if !agu_alloc_ok {
                println!("[SCH] Stalled for AGU scheduler allocation");
                println!("[SCH] Free AGSQ slots:  {:3} (need {})", 
                         num_agu_free, num_agu_alloc);
                break 'dispatch;
            }

            for uop in uops.iter_mut() {
                // Resolve all architectural source registers
                for arg in uop.arg.iter_mut() {
                    if let Storage::Arn(r) = arg {
                        let p = rat.resolve(*r);
                        println!("[SCH] Resolved {:?} to {:?}", r, p);
                        *arg = Storage::Prn(p);
                    }
                }

                // Allocate for architectural destination register
                for eff in uop.eff.iter_mut() {
                    if let Effect::RegWrite(rd, prn) = eff {
                        if prn == &Prn::alloc() {
                            let nprn = prf.alloc().unwrap();
                            println!("[SCH] Allocated {:?} for result {:?}", 
                                     nprn, rd);
                            //rat.bind(rd.clone(), nprn);
                            *eff = Effect::RegWrite(rd.clone(), nprn);
                        }
                    }
                }

                // Simultaneously, send this micro-op to a scheduler and
                // allocate an appropriate ROB entry
                //
                // NOTE: This doesn't make any "real" attempt to actually 
                // balance the ALU scheduling.
                match uop.kind {

                    UopKind::Alu(_) => {
                        // Naively prioritize the emptiest queue
                        let (i, mut tgt_alq) = alu_sched.iter_mut()
                            .enumerate().max_by(|(i,x),(j,y)| { 
                                x.num_free().cmp(&y.num_free()) 
                        }).unwrap();

                        let rob_ent = ROBEntry::new(mop, *uop);
                        let rob_idx = rob.push(rob_ent).unwrap();
                        println!("[SCH] ALSQ{} dispatch {:08x} {:?} rob_idx={} ", 
                                 i, uop.addr, uop.kind, rob_idx
                        );
                        tgt_alq.alloc(
                            Reservation { mop, uop: *uop, rob_idx }
                        ).unwrap();
                    },

                    UopKind::Agu(_) => {
                        let rob_ent = ROBEntry::new(mop, *uop);
                        let rob_idx = rob.push(rob_ent).unwrap();
                        println!("[SCH] AGSQ dispatch {:08x} {:?} rob_idx={} ", 
                                 uop.addr, uop.kind, rob_idx
                        );
                        agu_sched.alloc( 
                            Reservation { mop, uop: *uop, rob_idx }
                        ).unwrap();
                    },

                    // Let's assume that UD2 doesn't consume a scheduler entry
                    // and only lives as a marker in the ROB
                    UopKind::Illegal => {
                        let rob_ent = ROBEntry::new(mop, *uop);
                        let rob_idx = rob.push(rob_ent).unwrap();
                        println!("[SCH] Allocated ROB entry {} for uop", rob_idx);
                    },

                    _ => unreachable!(),
                }
            }

            // It's safe to finally pop this macro-op from the queue.
            opq.pop().unwrap();
        }
    }
}


