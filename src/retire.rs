use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};

use crate::op::*;
use crate::rf::*;
use crate::dispatch::*;
use crate::util::*;

/// An entry in the reorder buffer.
#[derive(Clone, Copy, Debug)]
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

pub struct ReorderBuffer {
    pub tag: usize,
    pub data: Queue<ROBEntry>,
}
impl ReorderBuffer {
    pub fn new() -> Self {
        Self {
            tag: 0,
            data: Queue::new(224),
        }
    }
    pub fn num_free(&self) -> usize { self.data.cap - self.data.len() }

    pub fn push(&mut self, e: ROBEntry) -> Result<usize, ()> {
        self.data.push(e)
    }
}
