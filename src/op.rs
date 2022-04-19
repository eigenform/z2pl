
use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};


use crate::rf::*;
use crate::front::DecodedInst;


#[derive(Debug, Copy, Clone)]
pub enum MacroOp {
    Nop, Ud2,

    MovRI(Register, i64),
    // Base, index, displacement, Width, Source
    MovMR(Register, Register, usize, MemorySize, Register),

    AluRI(ALUOp, Register, i64),
    AluRR(ALUOp, Register, Register),
    JmpI(usize),
}
impl MacroOp {
    pub fn uop_types(&self) -> [UopType; 2] {
        match self {
            Self::Nop => [UopType::Alu, UopType::None],
            Self::Ud2 => [UopType::None,UopType::None],
            Self::MovRI(..) => [UopType::Alu, UopType::None],
            Self::MovMR(..) => [UopType::St, UopType::None],
            Self::AluRI(..) => [UopType::Alu, UopType::None],
            Self::AluRR(..) => [UopType::Alu, UopType::None],
            Self::JmpI(..) =>  [UopType::Brn, UopType::None],
            _ => unimplemented!(),
        }
    }
    pub fn reg_result(&self) -> Option<Register> {
        match self {
            Self::MovRI(rd, _) | 
            Self::AluRI(_, rd, _) | 
            Self::AluRR(_, rd, _) => Some(*rd),

            Self::JmpI(_) => None,
            Self::Nop | Self::Ud2 => None,
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Storage { Reg(Prn), Imm64(i64), Bypass(usize), Zero }

#[derive(Clone, Copy, Debug)]
pub enum Uop {
    Nop, Ud,
    // op, dst, x, y
    Alu(ALUOp, Storage, Storage, Storage),
    // dst, src_addr, width
    Load(Storage, Storage, MemorySize), 
    // dst_addr, src_val, width
    Store(Storage, Storage, MemorySize),
}
#[derive(Clone, Copy, Debug)]
pub enum UopType {
    None, Rename, Brn, Ld, St, Alu,
}


#[derive(Debug, Copy, Clone)]
pub enum ALUOp { Add, Sub, Or, And, Xor, Shl, Shr }
#[derive(Debug, Copy, Clone)]
pub enum LSUOp { Load, Store }


pub fn get_macro_ops(dec: &DecodedInst) -> MacroOp {
    println!("{:08x}: {:?} {:02x?}", dec.addr, 
        dec.inst.code(), &dec.bytes[..dec.inst.len()]);

    let mut fac = InstructionInfoFactory::new();
    let info = fac.info(&dec.inst);

    let opcd = dec.inst.mnemonic();
    println!("{:?}", opcd);
    use iced_x86::Mnemonic::*;
    match opcd {
        Ud2 => MacroOp::Ud2, 
        Nop => MacroOp::Nop,
        Mov => {
            let dst = dec.inst.op0_kind();
            let src = dec.inst.op1_kind();
            println!("{:?} := {:?}", dst, src);
            match (dst, src) {
                (OpKind::Register, OpKind::Immediate32to64) => {
                    MacroOp::MovRI(
                        dec.inst.op0_register(), dec.inst.immediate32to64()
                    )
                },
                (OpKind::Memory, OpKind::Register) => {
                    let rbase = dec.inst.memory_base();
                    let ridx  = dec.inst.memory_index();
                    let disp  = dec.inst.memory_displacement64() as usize;
                    let sz    = dec.inst.memory_size();
                    MacroOp::MovMR(rbase, ridx, disp, sz, 
                        dec.inst.op1_register())
                },
                _ => unimplemented!("{:?} {:?}", dst, src),
            }
        },
        Add | Sub | And | Or | Xor => {
            let dst = dec.inst.op0_kind();
            let src = dec.inst.op1_kind();
            let aluop = match opcd {
                Add => ALUOp::Add,
                Sub => ALUOp::Sub,
                And => ALUOp::And,
                Or => ALUOp::Or,
                Xor => ALUOp::Xor,
                _ => unreachable!(),
            };
            match (dst, src) {
                (OpKind::Register, OpKind::Immediate32to64) => {
                    MacroOp::AluRI(aluop,
                        dec.inst.op0_register(), dec.inst.immediate32to64()
                    )
                },
                (OpKind::Register, OpKind::Register) => {
                    MacroOp::AluRR(aluop,
                        dec.inst.op0_register(), dec.inst.op1_register()
                    )
                }
                _ => unimplemented!("{:?} {:?}", dst, src),
            }

        },

        Jmp => {
            assert!(dec.inst.op0_kind() == OpKind::NearBranch64);
            let tgt = dec.inst.near_branch64();
            MacroOp::JmpI(tgt as usize)
        },
        _ => unimplemented!("{:?}", opcd),
    }

}



