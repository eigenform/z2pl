
use crate::mem::clk;
use crate::op::*;
use crate::issue::*;
use crate::retire::*;
use crate::rf::*;

pub struct ExecutionUnits {
    pub alu: [ALU; 4],
}
impl ExecutionUnits {
    pub fn new() -> Self {
        Self {
            alu: [ALU::new(); 4],
        }
    }
    pub fn cycle(&mut self, 
        rob: &mut ReorderBuffer, 
        prf: &mut PhysicalRegisterFile
    ) {

        for (idx, tgt_alu) in self.alu.iter_mut().enumerate() {
            let res = tgt_alu.cycle(rob, prf);
            match res {
                Ok(comp) => {
                    println!("[ALU] {:08x}: {:?}, rob_idx={} complete", 
                        comp.uop.addr, comp.uop.kind, comp.rob_idx);

                    rob.get_mut(comp.rob_idx).unwrap().complete = true;
                },
                Err(ALUErr::PendingCompletion) => {
                    let op = tgt_alu.op.unwrap();
                    println!("[ALU] {:08x}: {:?}", op.uop.addr, op.uop.kind);
                },
                Err(ALUErr::Empty) => {},
            }
        }
    }
}


pub enum ALUErr {
    PendingCompletion,
    Empty,
}

/// Arithmetic-logic unit.
///
#[derive(Debug, Copy, Clone)]
pub struct ALU {
    /// Micro-op currently occupying this ALU
    pub op: Option<Reservation>,
    /// The cycle number that an operation started on
    pub cycle_in: usize,
}
impl ALU {
    pub fn new() -> Self {
        Self { op: None, cycle_in: 0 }
    }

    pub fn busy(&self) -> bool { self.op.is_some() }

    pub fn cycle(&mut self, rob: &mut ReorderBuffer, 
                 prf: &mut PhysicalRegisterFile) 
        -> Result<Reservation, ALUErr>
    {
        if let Some(tgt) = self.op {
            // Determine if this operation needs to be completed this cycle
            // (according to our assumptions about micro-op latencies)
            if (clk() - self.cycle_in) >= tgt.uop.latency() {
                let alu_op = {
                    if let UopKind::Alu(alu_op) = tgt.uop.kind { alu_op }
                    else { unreachable!() }
                };

                // Short circuit for NOPs
                if alu_op == ALUOp::Nop || alu_op == ALUOp::Brn {
                    self.op = None;
                    return Ok(tgt);
                }

                let x = match tgt.uop.arg[0] {
                    Storage::Imm64(v)  => v as usize,
                    Storage::Zero      => 0,
                    Storage::Prn(rs)   => prf.read(rs),
                    Storage::Bypass(_) => unimplemented!(),
                    Storage::Arn(_)    => unreachable!(),
                    Storage::None      => unreachable!(),
                };
                let y = match tgt.uop.arg[1] {
                    Storage::Imm64(v)  => v as usize,
                    Storage::Zero      => 0,
                    Storage::Prn(rs)   => prf.read(rs),
                    Storage::Bypass(_) => unimplemented!(),
                    Storage::Arn(_)    => unreachable!(),
                    Storage::None      => unreachable!(),
                };

                // Perform the actual computation
                let res = match alu_op {
                    ALUOp::Add => x.wrapping_add(y),
                    _ => unimplemented!(),
                };

                // Phyiscal register file write 
                if let Effect::RegWrite(arn, prn) = tgt.uop.eff[0] {
                    println!("[ALU] PRF write {:016x} to {:?}", res, prn);
                    prf.write(prn, res);
                }

                self.op = None;
                Ok(tgt)
            } else {
                Err(ALUErr::PendingCompletion)
            }
        } else {
            Err(ALUErr::Empty)
        }
    }

    pub fn do_issue(&mut self, cyc: usize, tgt: Reservation) {
        assert!(self.op.is_none());
        self.op = Some(tgt);
        self.cycle_in = cyc;
    }
}
