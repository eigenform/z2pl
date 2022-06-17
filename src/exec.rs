
use crate::mem::clk;
use crate::op::*;
use crate::dispatch::*;
use crate::retire::*;
use crate::rf::*;

pub enum ALUErr {
    PendingCompletion,
    Empty,

}

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
            // (according to our assumptions about micro-op latencies) and
            // perform the actual computation.
            if (clk() - self.cycle_in) >= tgt.uop.latency() {
                let alu_op = {
                    if let UopKind::Alu(alu_op) = tgt.uop.kind { alu_op }
                    else { unreachable!() }
                };

                match alu_op {
                    ALUOp::Nop => {},
                    _ => unimplemented!(),
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
