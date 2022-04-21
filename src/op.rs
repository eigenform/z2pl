
use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
    Mnemonic, Register, Code, MemorySize
};

use crate::rf::*;
use crate::retire::*;
use crate::dispatch::*;
use crate::front::DecodedInst;

/// Representing a "macro-op".
#[derive(Debug, Copy, Clone)]
pub enum MacroOp {
    Nop, Ud2,
    /// Mov (register <- immediate)
    MovRI(Register, i64),
    /// Mov (memory <- register) (rd, ridx, disp, width, rs)
    MovMR(Register, Register, usize, MemorySize, Register),
    /// Alu (register <- immediate)
    AluRI(ALUOp, Register, i64),
    /// Alu (register <- register)
    AluRR(ALUOp, Register, Register),
    /// Jump (immediate)
    JmpI(usize),
}

/// Convert a decoded instruction into one [or more] macro-ops.
pub fn get_macro_ops(dec: &DecodedInst) -> MacroOp {
    println!("[IDU] {:08x}: {:?} {:02x?}", dec.addr, 
        dec.inst.code(), &dec.bytes[..dec.inst.len()]);
    let mut fac = InstructionInfoFactory::new();
    let info = fac.info(&dec.inst);
    let opcd = dec.inst.mnemonic();
    use iced_x86::Mnemonic::*;
    match opcd {
        Ud2 => MacroOp::Ud2, 
        Nop => MacroOp::Nop,
        Mov => {
            let dst = dec.inst.op0_kind();
            let src = dec.inst.op1_kind();
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

/// Storage locations for values in the back-end of the machine.
#[derive(Clone, Copy, Debug)]
pub enum Storage { 
    /// An architectural register (to-be-renamed).
    Arn(Register),
    /// A physical register.
    Prn(Prn),
    /// A signed 64-bit immediate value
    Imm64(i64), 
    /// Identifier for some bypass path
    Bypass(usize), 
    /// A value of zero
    Zero,
    None,
}

#[derive(Clone, Copy, Debug)]
pub enum ArchStorage {
    /// An architectural register.
    Reg(Register),
    /// A memory location.
    Mem(usize),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    RegWrite(Register, Prn),
    MemWrite(Prn, Prn),
    BrnImm(usize),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UopKind {
    None, Illegal, Alu(ALUOp), Agu(AGUOp)
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ALUOp { 
    Nop, Add, Sub, Or, And, Xor, Shl, Shr,
    Brn,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AGUOp { Ld(MemorySize), St(MemorySize), LdSt }

#[derive(Clone, Copy, Debug)]
pub struct Uop {
    /// Address associated with this micro-op
    pub addr: usize,
    /// The actual operation
    pub kind: UopKind,
    /// Input operands
    pub arg: [Storage; 5],
    /// Output operands and architectural effects
    pub eff: [Effect; 2],
}
impl Uop {
    pub fn empty(addr: usize) -> Self {
        Self { 
            addr,
            kind: UopKind::None, 
            arg: [Storage::None; 5],
            eff: [Effect::None; 2],
        }
    }
    pub fn preg_allocs(&self) -> usize {
        self.eff.iter().filter(|e| 
            if let Effect::RegWrite(_, prn) = e {
                if prn == &Prn::alloc() { true } else { false }
            } else { false }
        ).count()
    }
    pub fn is_alu(&self) -> bool {
        if let UopKind::Alu(_) = self.kind { true } else { false }
    }
    pub fn is_agu(&self) -> bool {
        if let UopKind::Agu(_) = self.kind { true } else { false }
    }


    pub fn from_mop(mop: MacroOp, addr: usize) -> Vec<Self> {
        let mut res = Vec::new();
        let mut op1 = Uop::empty(addr);
        let mut op2 = Uop::empty(addr);
        match mop {
            MacroOp::Nop => {
                op1.kind = UopKind::Alu(ALUOp::Nop);
                res.push(op1);
            },
            MacroOp::MovRI(rd, imm) => {
                op1.kind = UopKind::Alu(ALUOp::Add);
                op1.arg[0] = Storage::Imm64(imm);
                op1.arg[1] = Storage::Zero;
                op1.eff[0] = Effect::RegWrite(rd, Prn::alloc());
                res.push(op1);
            },
            MacroOp::MovMR(base, idx, disp, sz, src) => {
                op1.kind = UopKind::Agu(AGUOp::St(sz));
                op1.arg[0] = Storage::Arn(base);
                op1.arg[1] = Storage::Arn(idx);
                op1.arg[2] = Storage::Imm64(disp as i64);
                op1.arg[3] = Storage::Arn(src);
                res.push(op1);
            },
            MacroOp::AluRR(opcd, rd, rs) => {
                op1.kind = UopKind::Alu(opcd);
                op1.arg[0] = Storage::Arn(rd);
                op1.arg[1] = Storage::Arn(rs);
                op1.eff[0] = Effect::RegWrite(rd, Prn::alloc());
                res.push(op1);
            },
            MacroOp::JmpI(tgt_imm) => {
                op1.kind = UopKind::Alu(ALUOp::Brn);
                op1.eff[0] = Effect::BrnImm(tgt_imm);
                res.push(op1);
            },
            _ => unimplemented!("no uop decomposition for {:?}", mop),
        }
        res
    }
}

