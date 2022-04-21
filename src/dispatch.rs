
use crate::rf::*;
use crate::op::*;
use crate::retire::*;

/// An entry in the macro-op queue.
#[derive(Debug, Copy, Clone)]
pub struct OPQEntry {
    pub addr: usize,
    pub op: MacroOp,
}

/// Abstract representation of the dispatch unit.
pub struct DispatchUnit {
    /// Index of the next target ALU scheduler.
    /// This is used to implement simple round-robin ALU dispatch.
    pub next_alu_sched: usize,
}
impl DispatchUnit {
    pub fn new() -> Self {
        Self {
            next_alu_sched: 0,
        }
    }

}

/// Entry in an ALU scheduler.
#[derive(Clone, Copy, Debug)]
pub struct ALUReservation {
    pub mop: MacroOp,
    pub uop: Uop,
}

/// Entry in an AGU scheduler.
#[derive(Clone, Copy, Debug)]
pub struct AGUReservation {
    pub mop: MacroOp,
    pub uop: Uop,
}

pub trait Scheduler {
    type Reservation: Sized + Clone + Copy + std::fmt::Debug;
    fn can_alloc(&self) -> bool;
    fn can_allocn(&self, n: usize) -> bool;
    fn num_free(&self) -> usize;
    fn alloc(&mut self, e: Self::Reservation) -> Result<(), ()>;
}

/// An ALU scheduler/reservation station.
#[derive(Clone, Copy, Debug)]
pub struct ALUScheduler {
    pub data: [Option<ALUReservation>; 16],
}
impl ALUScheduler {
    pub fn new() -> Self {
        Self { data: [None; 16] }
    }
}
impl Scheduler for ALUScheduler {
    type Reservation = ALUReservation;
    fn can_alloc(&self) -> bool {
        self.data.iter().any(|&e| e.is_none()) 
    }
    fn can_allocn(&self, n: usize) -> bool {
        self.data.iter().filter(|&e| e.is_none()).count() >= n
    }
    fn num_free(&self) -> usize {
        self.data.iter().filter(|&e| e.is_none()).count()
    }
    fn alloc(&mut self, new: Self::Reservation) -> Result<(), ()> {
        if let Some((i, e)) = self.data.iter_mut().enumerate()
            .find(|(i, e)| e.is_none()) 
        {
            self.data[i] = Some(new);
            Ok(())
        } else { Err(()) }
    }
}

/// An AGU scheduler/reservation station.
#[derive(Clone, Copy, Debug)]
pub struct AGUScheduler {
    pub data: [Option<AGUReservation>; 28],
}
impl AGUScheduler {
    pub fn new() -> Self {
        Self { data: [None; 28] }
    }
}
impl Scheduler for AGUScheduler {
    type Reservation = AGUReservation;
    fn can_alloc(&self) -> bool {
        self.data.iter().any(|&e| e.is_none()) 
    }
    fn can_allocn(&self, n: usize) -> bool {
        self.data.iter().filter(|&e| e.is_none()).count() >= n
    }
    fn num_free(&self) -> usize {
        self.data.iter().filter(|&e| e.is_none()).count()
    }
    fn alloc(&mut self, new: Self::Reservation) -> Result<(), ()> {
        if let Some((i, e)) = self.data.iter_mut().enumerate()
            .find(|(i, e)| e.is_none()) 
        {
            self.data[i] = Some(new);
            Ok(())
        } else { Err(()) }
    }
}



