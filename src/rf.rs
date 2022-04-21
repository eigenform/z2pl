
use std::collections::HashMap;
use iced_x86::Register;

/// A tag for a physical register.
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



pub struct RegisterAliasTable {
    pub data: HashMap<Register, Prn>
}
impl RegisterAliasTable {
    pub fn new() -> Self {
        let mut data = HashMap::new();
        data.insert(Register::RAX, Prn(0));
        data.insert(Register::RBX, Prn(0));
        data.insert(Register::RCX, Prn(0));
        data.insert(Register::RDX, Prn(0));
        data.insert(Register::RSI, Prn(0));
        data.insert(Register::RDI, Prn(0));
        data.insert(Register::RBP, Prn(0));
        data.insert(Register::RSP, Prn(0));
        data.insert(Register::R8,  Prn(0));
        data.insert(Register::R9,  Prn(0));
        data.insert(Register::R10, Prn(0));
        data.insert(Register::R11, Prn(0));
        data.insert(Register::R12, Prn(0));
        data.insert(Register::R13, Prn(0));
        data.insert(Register::R14, Prn(0));
        data.insert(Register::R15, Prn(0));
        Self { data }
    }
    pub fn resolve(&self, r: &Register) -> Prn {
        *self.data.get(r).unwrap()
    }
    pub fn bind(&mut self, r: Register, prn: Prn) {
        self.data.insert(r, prn).unwrap();
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


