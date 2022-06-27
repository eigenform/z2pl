
use crate::mem::*;
use crate::op::*;
use crate::exec::*;

/// Entry in a scheduler.
#[derive(Clone, Copy, Debug)]
pub struct Reservation {
    pub mop: MacroOp,
    pub uop: Uop,
    pub rob_idx: usize,
}


/// A scheduler/reservation station for dispatched micro-ops.
#[derive(Clone, Copy, Debug)]
pub struct Scheduler<const SIZE: usize> {
    pub data: [Option<Reservation>; SIZE],
}
impl <const SIZE: usize> Scheduler<SIZE> {
    pub fn new() -> Self {
        Self { data: [None; SIZE] }
    }

    /// Returns true if there is at least one free slot.
    pub fn can_alloc(&self) -> bool {
        self.data.iter().any(|&e| e.is_none()) 
    }

    /// Returns true if there are at least 'n' free slots.
    pub fn can_allocn(&self, n: usize) -> bool {
        self.data.iter().filter(|&e| e.is_none()).count() >= n
    }

    /// Return the number of free slots.
    pub fn num_free(&self) -> usize {
        self.data.iter().filter(|&e| e.is_none()).count()
    }

    /// Return the number of pending slots.
    pub fn num_pending(&self) -> usize {
        self.data.iter().filter(|&e| e.is_some()).count()
    }

    /// Fill a slot in the scheduler.
    pub fn alloc(&mut self, new: Reservation) -> Result<(), ()> {
        if let Some((i, e)) = self.data.iter_mut().enumerate()
            .find(|(i, e)| e.is_none()) 
        {
            self.data[i] = Some(new);
            Ok(())
        } else { Err(()) }
    }

    /// Return the number of reservations which are ready-for-issue.
    pub fn num_ready(&self) -> usize {
        if self.num_pending() == 0 { return 0; }
        let pending_slots = self.data.iter().filter_map(|s| *s);
        pending_slots.filter(|res| res.uop.fire()).count()
    }

    // Find and return a reservation which is ready-for-issue, removing the 
    // reservation from the scheduler queue.
    pub fn take_ready(&mut self) -> Option<Reservation> {
        if self.num_pending() == 0 {
            return None;
        }

        let mut iter = self.data.iter_mut();
        while let Some(slot) = iter.next() {
            if let Some(entry) = slot {
                if entry.uop.fire() {
                    return slot.take();
                }
            }
        }
        None
    }
}

/// A 16-entry ALU scheduler.
pub type ALUScheduler = Scheduler<16>;

/// A 28-entry AGU scheduler.
pub type AGUScheduler = Scheduler<28>;


pub struct IssueUnit;
impl IssueUnit {
    pub fn cycle(&mut self, alu_sched: &mut [ALUScheduler; 4], 
                 eu: &mut ExecutionUnits)
    {
        // Iterate over all ALU schedulers and attempt to fire any pending
        // reservations that are ready-for-issue.
        //
        // Each ALQ can only issue 1 micro-op per cycle.

        let mut free_alus = eu.alu.iter_mut().enumerate()
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
    }
}
