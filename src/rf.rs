
use std::collections::HashMap;
use iced_x86::Register;

/// A tag for a physical register.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Prn(pub usize);
impl Prn {
    pub fn alloc() -> Self { Prn(usize::MAX) }
}
impl std::fmt::Debug for Prn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == usize::MAX { 
            write!(f, "Prn(NA)")
        } else {
            write!(f, "Prn({})", self.0)
        }
    }
}

/// A tag for an architectural register.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Arn(pub usize);
impl std::fmt::Debug for Arn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", Register::from(*self))
    }
}

impl From<Arn> for Register {
    fn from(x: Arn) -> Self {
        match x.0 {
            00 => Register::RAX,
            01 => Register::RBX,
            02 => Register::RCX,
            03 => Register::RDX,
            04 => Register::RSI,
            05 => Register::RDI,
            06 => Register::RBP,
            07 => Register::RSP,
            08 => Register::R8,
            09 => Register::R9,
            10 => Register::R10,
            11 => Register::R11,
            12 => Register::R12,
            13 => Register::R13,
            14 => Register::R14,
            15 => Register::R15,
            _ => unimplemented!(),

        }
    }
}
impl From<Register> for Arn {
    fn from(x: Register) -> Self {
        let num = match x {
            Register::RAX => 00,
            Register::RBX => 01,
            Register::RCX => 02,
            Register::RDX => 03,
            Register::RSI => 04,
            Register::RDI => 05,
            Register::RBP => 06,
            Register::RSP => 07,
            Register::R8  => 08,
            Register::R9  => 09,
            Register::R10 => 10,
            Register::R11 => 11,
            Register::R12 => 12,
            Register::R13 => 13,
            Register::R14 => 14,
            Register::R15 => 15,
            _ => unimplemented!(),
        };
        Self(num)
    }
}

pub struct RegisterAliasTable {
    //pub data: HashMap<Register, Prn>
    pub data: [Prn; 16],
}
impl RegisterAliasTable {
    pub fn new() -> Self {
        let mut data: [Prn; 16] = [Prn(0); 16];
        Self { data }
    }
    pub fn print(&self, prf: &PhysicalRegisterFile) {
        println!("[RAT] Register Alias Table state:");
        for (arn, prn) in self.data.iter().enumerate() {
            let areg = format!("{:?}", Register::from(Arn(arn)));
            println!("[RAT]   {:3} => {:03} => {:016x}", 
                     areg, prn.0, prf.read(*prn));
        }
    }
    pub fn resolve(&self, r: Register) -> Prn {
        let idx = Arn::from(r).0;
        self.data[idx]
    }

    pub fn update(&mut self, r: Register, prn: Prn) {
        let idx = Arn::from(r).0;
        self.data[idx] = prn;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PRFEntry {
    pub free: bool,
    pub data: usize,
}
impl PRFEntry {
    pub fn new() -> Self {
        Self { free: true, data: 0 }
    }
}

pub struct PhysicalRegisterFile {
    pub data: [PRFEntry; 180],
}
impl PhysicalRegisterFile {
    pub fn new() -> Self {
        let mut res = Self { data: [PRFEntry::new(); 180] };
        // NOTE: The initial RAT maps all registers to Prn(0)
        res.alloc_explicit(Prn(0)).unwrap();
        res
    }
    pub fn can_alloc(&self) -> bool {
        self.data.iter().any(|&e| e.free)
    }
    pub fn can_allocn(&self, n: usize) -> bool {
        self.data.iter().filter(|&e| e.free).count() >= n
    }
    pub fn free_regs(&self) -> usize {
        self.data.iter().filter(|&e| e.free).count()
    }

    pub fn find(&mut self) -> Option<Prn> {
        if let Some((i, e)) = self.data.iter_mut().enumerate()
            .find(|(i, e)| e.free) 
        {
            Some(Prn(i))
        } else { None }
    }

    pub fn read(&self, prn: Prn) -> usize {
        assert!(self.data[prn.0].free == false);
        self.data[prn.0].data
    }
    pub fn write(&mut self, prn: Prn, val: usize) {
        assert!(self.data[prn.0].free == false);
        self.data[prn.0].data = val;
    }


    pub fn alloc(&mut self) -> Option<Prn> {
        if let Some(prn) = self.find() { 
            self.alloc_explicit(prn).unwrap();
            Some(prn)
        } else {
            None
        }
    }

    /// Explicitly allocate a particular physical register.
    pub fn alloc_explicit(&mut self, prn: Prn) -> Result<(), ()> {
        assert!(self.data[prn.0].free == true);
        self.data[prn.0].free = false;
        Ok(())
    }

    /// Explicitly clear and free a particular physical register.
    pub fn free_explicit(&mut self, prn: Prn) {
        assert!(self.data[prn.0].free == false);
        self.data[prn.0].free = true;
        self.data[prn.0].data = 0;
    }

}
impl std::ops::Index<usize> for PhysicalRegisterFile {
    type Output = PRFEntry;
    fn index(&self, x: usize) -> &Self::Output {
        assert!(x < self.data.len());
        &self.data[x]
    }
}
impl std::ops::IndexMut<usize> for PhysicalRegisterFile {
    fn index_mut(&mut self, x: usize) -> &mut Self::Output {
        assert!(x < self.data.len());
        &mut self.data[x]
    }
}


