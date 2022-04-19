
use iced_x86::Instruction;



pub enum MacroOp {
}

pub struct MopData {
    uop: [Option<Uop>; 2],
}



pub enum UopType {
    Load, Store, LoadStore,
    MovImm, Add,
}


#[derive(Copy, Clone)]
pub struct Uop {
    pub addr: usize,
    pub inst: Instruction,
}


