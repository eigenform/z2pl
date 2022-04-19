
use crate::util::*;
use crate::mem::*;
use crate::op::*;
use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};


/// A 64-byte cache line.
#[derive(Copy, Clone)]
pub struct CacheLine { pub addr: usize, pub data: [u8; 64] }
/// A 32-byte cache half-line.
#[derive(Copy, Clone)]
pub struct HalfLine { pub addr: usize, pub data: [u8; 32] }
/// Entry in the instruction byte queue.
#[derive(Copy, Clone)]
pub struct IBQEntry { pub addr: usize, pub data: [u8; 16] }

/// Instruction fetch logic.
pub struct FetchUnit;
impl FetchUnit {
    pub fn cycle(&mut self, ftq: &mut Queue<usize>, ibq: &mut Queue<IBQEntry>) {
        let addr = ftq.pop().unwrap();
        let data = cache_read(addr);
        ibq.push(IBQEntry { 
            addr: addr + 0x00, data: data[0x00..0x10].try_into().unwrap() 
        }).unwrap();
        ibq.push(IBQEntry { 
            addr: addr + 0x10, data: data[0x10..].try_into().unwrap() 
        }).unwrap();
    }
}

#[derive(Clone, Copy)]
pub struct DecodedInst {
    pub inst: Instruction,
    pub bytes: [u8; 0x10],
    pub addr: usize,
}

/// Instruction decode logic.
pub struct DecodeUnit {
    /// Rolling cursor into the pick window
    pub pick_offset: usize,
}
impl DecodeUnit {
    pub fn cycle(&mut self, ibq: &mut Queue<IBQEntry>) 
        -> [Option<DecodedInst>; 4]
    {
        // Build the pick window
        let mut cursor = self.pick_offset;
        let mut pick   = [0u8; 32];
        let (bot, top) = (ibq.peek(0).unwrap(), ibq.peek(1).unwrap());
        let pick_addr  = bot.addr;
        pick[0x00..0x10].copy_from_slice(&bot.data);
        pick[0x10..].copy_from_slice(&top.data);

        let mut res: [Option<DecodedInst>; 4] = [None; 4];
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
            res[idx] = Some(DecodedInst { inst, bytes, addr });
            cursor += inst.len();
        }

        // Adjust the pick window for the next cycle
        match cursor {
            // Haven't finished decoding the head entry, roll over cursor
            0x00..=0x0f => { self.pick_offset = cursor; },
            // Finished first entry: pop first entry and roll over cursor
            0x10..=0x1f => {
                self.pick_offset = cursor - 0x10;
                ibq.pop().unwrap();
            },
            // Exhausted the whole window: reset cursor and pop both entries
            0x20 => {
                self.pick_offset = 0;
                ibq.popn_exact(2).unwrap();
            }
            _ => unreachable!(),
        }
        res
    }
}


#[derive(Debug, Copy, Clone)]
pub enum MacroOp {
    AddReg(Register, Register),
    MovImm(Register, i64),
    MovReg(Register, Register),
    JmpImm(usize),
    StoreReg(Register, Register, u64, Register, MemorySize),
    Nop,
    Ud2,
}

#[derive(Debug, Copy, Clone)]
pub struct OPQEntry {
    pub addr: usize,
    pub op: MacroOp,
}


pub fn get_macro_ops(inst: &DecodedInst) -> MacroOp {
    println!("{:08x}: {:?} {:02x?}", inst.addr, 
        inst.inst.code(), &inst.bytes[..inst.inst.len()]);

    //for op in inst.inst.op_kinds() { println!("{:?}", op); }

    use iced_x86::Code::*;
    let mut fac = InstructionInfoFactory::new();
    let info = fac.info(&inst.inst);

    match inst.inst.code() {
        Ud2 => MacroOp::Ud2,
        Nop_rm16 | Nopw => MacroOp::Nop,
        Mov_rm64_imm32 => {
            match inst.inst.op0_kind() {
                OpKind::Register => {
                    let rd  = inst.inst.op0_register();
                    let imm = inst.inst.immediate32to64();
                    MacroOp::MovImm(rd, imm)
                }
                _ => unimplemented!(),
            }
        },
        Mov_rm64_r64 => {
            let rs = inst.inst.op1_register();
            match inst.inst.op0_kind() {
                OpKind::Register => {
                    let rd  = inst.inst.op0_register();
                    MacroOp::MovReg(rd, rs)
                },
                OpKind::Memory => {
                    let rbase = inst.inst.memory_base();
                    let ridx  = inst.inst.memory_index();
                    let disp  = inst.inst.memory_displacement64();
                    let sz    = inst.inst.memory_size();
                    MacroOp::StoreReg(rbase, ridx, disp, rs, sz)
                },
                _ => unimplemented!(),
            }
        },
        Add_rm64_r64 => {
            let rs = inst.inst.op1_register();
            match inst.inst.op0_kind() {
                OpKind::Register => {
                    let rd = inst.inst.op0_register();
                    MacroOp::AddReg(rd, rs)
                },
                _ => unimplemented!(),
            }
        },
        Jmp_rel8_64 => {
            let tgt = inst.inst.near_branch64();
            println!("tgt={:x}", tgt);
            MacroOp::JmpImm(tgt as usize)
        }
        _ => panic!("Unknown macro-op for {:?}", inst.inst.code()),
    }

}



