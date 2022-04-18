use std::collections::{ VecDeque };

pub struct Queue<T: Sized + Clone + Copy> {
    pub data: VecDeque<T>,
    pub cap: usize,
}
impl <T: Sized + Clone + Copy> Queue<T> {
    pub fn new(cap: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(cap),
            cap,
        }
    }
    pub fn is_full(&self) -> bool { self.data.len() == self.cap }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn peek(&self, n: usize) -> Result<&T, ()> {
        if let Some(x) = self.data.get(n) { Ok(x) } else { Err(()) }
    }

    pub fn push(&mut self, e: T) -> Result<usize, ()> {
        if self.data.len() + 1 > self.data.capacity() {
            Err(())
        } else {
            self.data.push_back(e);
            Ok(self.data.len())
        }
    }
    pub fn pushn(&mut self, list: &[T]) -> Result<usize, ()> {
        if self.data.len() + list.len() > self.data.capacity() {
            Err(())
        } else {
            for e in list {
                self.push(*e).unwrap();
            }
            Ok(self.data.len())
        }
    }

    pub fn pop(&mut self) -> Result<T, ()> {
        if let Some(res) = self.data.pop_front() { Ok(res) } else { Err(()) }
    }
    pub fn popn(&mut self, num: usize) -> Result<Vec<T>, ()> {
        if self.is_empty() || self.len() < num {
            Err(())
        } else {
            let mut res = vec![None; num];
            for i in 0..(num-1) {
                res[i] = Some(self.pop().unwrap());
            }
            Ok(res.iter().map(|e| e.unwrap()).collect())
        }
    }
}

impl std::fmt::Debug for Queue<usize> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x?}", self.data)
    }
}
impl <A: std::fmt::Debug + Clone + Copy, B: Clone + Copy + IntoIterator + std::fmt::Debug> 
    std::fmt::Debug for Queue<(A, B)> 
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: Vec<A> = self.data
            .iter().map(|e| e.0).collect();
        write!(f, "{:x?}", x)
    }
}


