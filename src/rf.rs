


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

#[derive(Copy, Clone, Debug)]
pub struct Prn(pub usize);
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
    pub fn find(&mut self) -> Option<Prn> {
        if let Some((i, e)) = self.data.iter_mut().enumerate()
            .find(|(i, e)| e.free) 
        {
            Some(Prn(i))
        } else { None }
    }
    pub fn alloc(&mut self, prn: Prn) -> Result<(), ()> {
        assert!(self.data[prn.0].free == true);
        self.data[prn.0].free = false;
        Ok(())

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


