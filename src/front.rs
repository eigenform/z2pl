
use std::collections::BTreeMap;

use crate::util::*;
use crate::mem::*;
use crate::op::*;
use crate::dispatch::*;
use crate::rf::*;
use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};

pub struct NextPCLogic;
impl NextPCLogic {
    pub fn cycle(&mut self, 
        pc: &mut usize,
        pq: &mut Queue<usize>, 
        ftq: &mut Queue<usize>
    ) {

        if ftq.is_full() {
            println!("[NPC] Stalled for full FTQ");
            return; 
        }

        // If a prediction is queued up, send it to the FTQ
        if let Ok(p) = pq.pop() {
            println!("[FTQ] Using predicted address {:08x}", p);
            ftq.push(p).unwrap();
            *pc = p;
        } 
        // Otherwise, send the next-sequential fetch block address
        else {
            println!("[FTQ] Using next-sequential address {:08x}", pc);
            ftq.push(*pc).unwrap();
            *pc += 0x20;
        }
    }
}


/// A 64-byte cache line.
#[derive(Copy, Clone)]
pub struct CacheLine { pub addr: usize, pub data: [u8; 64] }

/// Entry in the instruction byte queue.
#[derive(Copy, Clone)]
pub struct IBQEntry { pub addr: usize, pub data: [u8; 16] }

/// Abstract representation of the instruction fetch unit.
///
/// NOTE: For now, we're assuming that the fetch unit *always* pushes the
/// whole window into the IBQ; otherwise, if there's no room for both entries,
/// the fetch unit is stalled. 
pub struct FetchUnit;
impl FetchUnit {
    pub fn cycle(&mut self, ftq: &mut Queue<usize>, ibq: &mut Queue<IBQEntry>) {

        // NOTE: Right now we assume that the fetch unit *always* pushes
        // two entries onto the IBQ (otherwise, if there aren't two free
        // entries in the IBQ, we stall the fetch unit).
        //
        // Also, is it 32B-per-cycle with SMT, or in single-threaded too?

        if ibq.num_free() < 2 {
            println!("[IFU] Stalled for full IBQ");
            return;
        }
        if ftq.is_empty() {
            println!("[IFU] Stalled for empty FTQ");
            return;
        }

        // Consume an entry from the FTQ and read 32 bytes from the L1 cache
        // per cycle. Push the resulting bytes onto the IBQ.

        let addr = ftq.pop().unwrap();
        let data = cache_read(addr);
        println!("[IFU] Fetching 32b at {:08x}", addr);
        ibq.push(IBQEntry { 
            addr: addr + 0x00, data: data[0x00..0x10].try_into().unwrap() 
        }).unwrap();
        ibq.push(IBQEntry { 
            addr: addr + 0x10, data: data[0x10..].try_into().unwrap() 
        }).unwrap();
        println!("[IFU] Pushed IBQ entry {:08x}", addr + 0x00);
        println!("[IFU] Pushed IBQ entry {:08x}", addr + 0x10);
    }
}

/// Representing a decoded instruction.
#[derive(Clone, Copy)]
pub struct DecodedInst {
    /// The address of this instruction
    pub addr: usize,
    /// Bytes (up to 16) associated with this instruction
    pub bytes: [u8; 0x10],
    /// x86 instruction metadata
    pub inst: Instruction,
}
impl DecodedInst {
    /// Get the 32B-aligned fetch address associated with this instruction.
    pub fn fetch_addr(&self) -> usize {
        self.addr & !0x0000_0000_0000_001f
    }
}

pub enum DecodeErr {
    OPQFull,
    IBQUnderflow,
}

/// Abstract representation of the instruction decode unit.
pub struct DecodeUnit {
    /// Rolling cursor (register) into the pick window.
    pub pick_offset: usize,
}
impl DecodeUnit {
    pub fn cycle(&mut self, 
        ibq: &mut Queue<IBQEntry>, 
        opq: &mut Queue<OPQEntry>,
        bpu: &mut BranchPredictionUnit,
    ) {
        use Mnemonic::*;

        if opq.is_full() {
            println!("[IDU] Stalled for full OPQ");
            return;
        }
        if ibq.len() < 2 {
            println!("[IDU] Stalled for IBQ entries");
            return;
        }

        // Build the pick window
        let mut cursor = self.pick_offset;
        let mut pick   = [0u8; 32];
        let (bot, top) = (ibq.peek(0).unwrap(), ibq.peek(1).unwrap());
        let pick_addr  = bot.addr;
        pick[0x00..0x10].copy_from_slice(&bot.data);
        pick[0x10..].copy_from_slice(&top.data);
        println!("[IDU] Decode started at pick window offset {:02x}", cursor);

        let mut output: [Option<DecodedInst>; 4] = [None; 4];
        let mut inst = Instruction::default();
        let mut decoder = Decoder::with_ip(
            64, &pick[cursor..], (pick_addr + cursor) as u64, 
            DecoderOptions::NONE
        );

        // Decode up to four instructions
        for idx in 0..4 {
            decoder.decode_out(&mut inst);
            if idx != 0 && inst.len() > 8 { break; }
            if inst.is_invalid() { break; }
            let mut bytes = [0u8; 0x10];
            bytes[..inst.len()]
                .copy_from_slice(&pick[cursor..(cursor + inst.len())]);
            let addr = pick_addr + cursor;
            output[idx] = Some(DecodedInst { inst, bytes, addr });
            cursor += inst.len();
        }

        // If the OPQ can't accept all of the decoded instructions,
        // we need to stall until some entries are free?
        let num_inst = output.iter().filter_map(|i| *i).count();
        if opq.num_free() < num_inst {
            println!("[IDU] Stall for OPQ entries");
            return; 
        }

        // Adjust the pick window for the next cycle
        match cursor {
            // Haven't finished decoding the head entry, roll over cursor
            0x00..=0x0f => { self.pick_offset = cursor; },
            // Finished first entry: pop first entry and roll over cursor
            0x10..=0x1f => {
                self.pick_offset = cursor - 0x10;
                println!("[IDU] Decode popped IBQ entry {:08x}", bot.addr);
                ibq.pop().unwrap();
            },
            // Exhausted the whole window: reset cursor and pop both entries
            0x20 => {
                self.pick_offset = 0;
                println!("[IDU] Decode popped IBQ entry {:08x}", bot.addr);
                println!("[IDU] Decode popped IBQ entry {:08x}", top.addr);
                ibq.popn_exact(2).unwrap();
            }
            _ => unreachable!(),
        }

        // Scan over all decoded instructions for this cycle
        for inst in output.iter().filter_map(|i| *i) {

            // Create a new entry in the OPQ
            let mop = get_macro_ops(&inst);
            let opq_entry = OPQEntry { op: mop, addr: inst.addr };
            opq.push(opq_entry).unwrap();

            // If this is a branch instruction, send it to the BPU
            let mn = inst.inst.mnemonic();
            match mn {
                Jmp | Jmpe | Jne | Je | Jge | Jle | Call | Ret => {
                    println!("[IDU] Encountered branch {:?}", mn);
                    bpu.push_branch(inst);
                },
                _ => {},
            }

        }

    }
}



pub struct BranchPredictionUnit {
    pub branches: Queue<DecodedInst>,
}
impl BranchPredictionUnit {
    pub fn new() -> Self {
        Self {
            branches: Queue::new(32),
        }
    }

    pub fn push_branch(&mut self, d: DecodedInst) {
        self.branches.push(d).unwrap();
    }

    // NOTE: How many branches can be predicted per-cycle?
    pub fn cycle(&mut self,
        btb: &mut BranchTargetBuffer,
        pq: &mut Queue<usize>,
    ) {

        if let Ok(brn) = self.branches.pop() {
            let fetch_addr = brn.fetch_addr();
            let info = BranchInfo { 
                kind: BranchKind::from(get_macro_ops(&brn)),
                bytes: brn.bytes,
                len: brn.inst.len(),
                addr: brn.addr,
            };

            // If a BTB entry exists for this fetch block
            if let Some(e) = btb.get_mut(fetch_addr) {

                // If this branch matches the entry
                if e.info == info {
                    if let Some(tgt) = e.tgt {
                        println!("[BPU] Predicted {:08x} for {:08x} {:?}", 
                                 tgt, info.addr, info.kind);
                        pq.push(tgt).unwrap();
                    } 
                    else 
                    {
                        println!("[BPU] No prediction for {:08x} {:?}", 
                                 info.addr, info.kind);
                    }
                } 
                else 
                {
                    println!("[BPU] Invalidate BTB entry {:08x}", fetch_addr);
                    e.info = info;
                    e.tgt  = None;
                }


            } else {
                println!("[BPU] No BTB entry for {:08x}", fetch_addr);
                btb.create(fetch_addr, info);
                println!("[BPU] Created new BTB entry");
            }

        } else {
            println!("[BPU] No branches to predict this cycle");
        }
    }
}


#[derive(Debug, PartialEq, Eq)]
pub struct BranchInfo {
    kind: BranchKind,
    bytes: [u8; 0x10],
    len: usize,
    addr: usize,
}
impl Default for BranchInfo {
    fn default() -> Self {
        Self {
            kind: BranchKind::None,
            bytes: [0; 0x10],
            len: 0,
            addr: 0,
        }
    }
}


/// Different kinds of x86 branch instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchKind {
    None,
    UnconditionalDirect,
    UnconditionalIndirect,
    ConditionalDirect,
    Call,
    Return,
}
impl From<MacroOp> for BranchKind {
    fn from(x: MacroOp) -> Self {
        match x {
            MacroOp::JmpI(_) => Self::UnconditionalDirect,
            _ => Self::None,
        }
    }
}


pub enum BTBErr { NotBranch, Miss }

pub struct BTBEntry {
    info: BranchInfo,
    tgt: Option<usize>,
}
impl Default for BTBEntry {
    fn default() -> Self {
        Self { 
            info: BranchInfo::default(),
            tgt: None,
        }
    }
}

pub struct BranchTargetBuffer {
    data: BTreeMap<usize, BTBEntry>,
}
impl BranchTargetBuffer {
    pub fn new() -> Self {
        Self { data: BTreeMap::new() }
    }

    /// Returns true if an entry exists for the provided fetch block. 
    pub fn hit(&self, fetch_pc: usize) -> bool {
        self.data.contains_key(&fetch_pc)
    }

    pub fn invalidate(&mut self, fetch_pc: usize) {
        self.data.remove(&fetch_pc);
    }

    /// Create a new BTB entry 
    pub fn create(&mut self, fetch_pc: usize, info: BranchInfo) {
        let e = BTBEntry { info, tgt: None };
        self.data.insert(fetch_pc, e);
    }

    /// Get a reference to some BTB entry
    pub fn get(&self, fetch_pc: usize) -> Option<&BTBEntry> {
        self.data.get(&fetch_pc)
    }

    /// Get a mutable reference to some BTB entry
    pub fn get_mut(&mut self, fetch_pc: usize) -> Option<&mut BTBEntry> {
        self.data.get_mut(&fetch_pc)
    }


}



