use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};

use crate::op::*;
use crate::rf::*;
use crate::dispatch::*;
use crate::util::*;

pub struct RetireControlUnit;
impl RetireControlUnit {
    pub fn cycle(&mut self, 
        rob: &mut ReorderBuffer, 
        rat: &mut RegisterAliasTable
    ) {
        println!("[RCU] Reorder buffer status:");
        println!("[RCU]   In-flight:    {}", rob.num_used());
        println!("[RCU]   Free entries: {}", rob.num_free());
        println!("[RCU]   Retire ptr:   {}", rob.retire_ptr);
        println!("[RCU]   Dispatch ptr: {}", rob.dispatch_ptr);

        for i in 0..8 {
            match rob.pop() {
                Ok((idx, ent)) => {
                    println!("[RCU] Retiring entry {} ({}/8): {:08x} {:?}",
                             idx, i, ent.uop.addr, ent.uop.kind);

                    // Commit architectural effects
                    for eff in ent.uop.eff {
                        match eff {
                            Effect::RegWrite(arn, prn) => {
                                rat.update(arn, prn);
                                println!("[RCU] {:?} commit to {:?}", prn, arn);
                            },
                            Effect::None => {},
                            _ => unimplemented!("{:x?}", eff),
                        }
                    }
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
    }
}


#[derive(Debug)]
pub enum ROBErr {
    Incomplete,
    Empty,
    Full,
}

/// An entry in the reorder buffer.
#[derive(Clone, Debug)]
pub struct ROBEntry {
    pub mop: MacroOp,
    pub uop: Uop,
    pub complete: bool,
}
impl ROBEntry {
    pub fn new(mop: MacroOp, uop: Uop) -> Self {
        Self { mop, uop, complete: false }
    }
}

//pub struct ReorderBuffer {
//    pub data: Queue<ROBEntry>,
//}
//impl ReorderBuffer {
//    pub fn new() -> Self {
//        Self { data: Queue::new(224) }
//    }
//    pub fn get_mut(&mut self, idx: usize) -> &mut ROBEntry {
//        self.data.get_mut(idx)
//    }
//
//
//    pub fn retire(&mut self) -> Result<ROBEntry, ROBErr> {
//        if let Some(ent) = self.data.front() {
//            if ent.complete { 
//                Ok(self.data.pop().unwrap())
//            } else {
//                Err(ROBErr::Incomplete)
//            }
//        } else {
//            Err(ROBErr::Empty)
//        }
//    }
//
//    pub fn front(&self) -> Option<&ROBEntry> { self.data.front() }
//    pub fn num_free(&self) -> usize { self.data.cap - self.data.len() }
//    pub fn push(&mut self, e: ROBEntry) -> Result<usize, ()> {
//        self.data.push(e)
//    }
//}

pub struct ReorderBuffer {
    data: Vec<Option<ROBEntry>>,
    pub retire_ptr: usize,
    pub dispatch_ptr: usize,
    pub size: usize,
}
impl ReorderBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            size, retire_ptr: 0, dispatch_ptr: 0, data: vec![None; size]
        }
    }

    pub fn num_used(&self) -> usize {
        self.data.iter().filter(|e| e.is_some()).count()
    }

    pub fn num_free(&self) -> usize {
        self.data.iter().filter(|e| e.is_none()).count()
    }

    pub fn is_full(&self) -> bool {
        (self.retire_ptr == self.dispatch_ptr) &&
            self.data[self.retire_ptr].is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.data[self.retire_ptr].is_none()
    }

    pub fn push(&mut self, e: ROBEntry) -> Result<usize, ROBErr> {
        if self.is_full() {
            Err(ROBErr::Full)
        } else {
            let res_ptr = self.dispatch_ptr;
            self.data[self.dispatch_ptr] = Some(e);
            self.dispatch_ptr = (self.dispatch_ptr + 1) % self.size;
            Ok(res_ptr)
        }
    }

    pub fn pop(&mut self) -> Result<(usize, ROBEntry), ROBErr> {
        if self.is_empty() {
            Err(ROBErr::Empty)
        } else {
            let retire_ptr = self.retire_ptr;
            if self.data[self.retire_ptr].as_ref().unwrap().complete {
                let res = self.data[self.retire_ptr].take().unwrap();
                self.retire_ptr = (self.retire_ptr + 1) % self.size;
                Ok((retire_ptr, res))
            } else {
                Err(ROBErr::Incomplete)
            }
        }
    }

    pub fn get_front(&self) -> Option<&ROBEntry> {
        self.data[self.retire_ptr].as_ref()
    }

    pub fn get(&self, idx: usize) -> Option<&ROBEntry> {
        assert!(idx < self.size);
        self.data[idx].as_ref()
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut ROBEntry> {
        assert!(idx < self.size);
        self.data[idx].as_mut()
    }

}



