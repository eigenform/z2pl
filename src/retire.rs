use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};

use crate::op::*;
use crate::rf::*;
use crate::schedule::*;


#[derive(Clone, Copy, Debug)]
pub struct ROBEntry {
    pub addr: usize,
    pub mop: MacroOp,
    pub uop: Uop,
    pub dst: Option<(Register, Prn)>,
    pub sched: SchedulerId,
    pub complete: bool,
}


