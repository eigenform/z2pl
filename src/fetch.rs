
use crate::util::*;

enum FetchUnitStatus {
    FetchTargetStall,
    L1QueueStall,
    Busy,
    Available
}


pub struct FetchUnit {
    pub tgt_q: Queue<usize>,
    pub l1_q:  Queue<usize>,
    pub mem:   Vec<u8>,
}
impl FetchUnit {
    pub fn new() -> Self {
        let mut mem = vec![0x90u8; 0x0001_0000];
        let mut tgt_q = Queue::new(8);
        let l1_q  = Queue::new(8);
        tgt_q.push(0).unwrap();
        Self { tgt_q, l1_q, mem }
    }
}

//    // Consume a fetch target address and send to L1 queue
//
//    let fetch_tgt = fetch_q.pop();
//    if let Ok(addr) = fetch_tgt {
//        l1_q.push(addr);
//        l1_q.push(addr + 0x20);
//    }
//
//    // Consume an L1, producing two byte queue entries
//
//    let l1_tgt = l1_q.pop();
//    if let Ok(addr) = l1_tgt {
//        let mut w0 = [0u8; 16];
//        let mut w1 = [0u8; 16];
//        w0.copy_from_slice(&mem[addr..addr+0x10]);
//        w1.copy_from_slice(&mem[addr+0x10..addr+0x20]);
//        byte_q.push((addr, w0));
//        byte_q.push((addr+0x10, w1));
//    }
//

