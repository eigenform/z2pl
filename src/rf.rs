

pub enum GPR {
    Rax, Rbx, Rcx, Rdx, Rsi, Rdi, Rsp, Rbp,
    R8, R9, R10, R11, R12, R13, R14, R15,
}


#[derive(Copy, Clone)]
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
        Self { 
            data: [PRFEntry::new(); 180] 
        }
    }
    pub fn can_alloc(&self) -> bool {
        self.data.iter().find(|&e| e.free).is_some()
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


