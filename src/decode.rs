use crate::util::*;

use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter,
    ConditionCode, InstructionInfoFactory, OpKind, RflagsBits,
};


//    // Pick
//
//    let mut byte_q: Queue<(usize, [u8; 16])> = Queue::new(20);
//    if byte_q.len() >= 2 {
//        let w0 = byte_q.pop().unwrap();
//        let w1 = byte_q.pop().unwrap();
//
//        let mut data = [0u8; 32];
//        data[0x00..0x10].copy_from_slice(&w0.1);
//        data[0x10..].copy_from_slice(&w0.1);
//
//        let mut decoder = Decoder::with_ip(64, &data, 0, DecoderOptions::NONE);
//        let mut inst = Instruction::default();
//        let mut inf = InstructionInfoFactory::new();
//        let mut res = Vec::new();
//
//        while decoder.can_decode() {
//            decoder.decode_out(&mut inst);
//            //let info = inf.info(&inst);
//            //let offsets = decoder.get_constant_offsets(&inst);
//            let start_index = (inst.ip() - 0) as usize;
//            let instr_bytes = &data[start_index..start_index + inst.len()];
//            println!("{:04x}: {:02x?} {:?}", inst.ip(),instr_bytes, inst.code());
//            res.push(inst);
//            if res.len() == 4 {
//                break
//            }
//        }
//
//        println!("Decode window [{:x?}, {:x?}]", w0.0, w1.0);
//    }


