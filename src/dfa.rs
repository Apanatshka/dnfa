extern crate bit_vec;

use self::bit_vec::BitVec;

pub const DFA_STUCK: usize = 0;
pub const DFA_START: usize = 1;

pub struct DFAState {
    pub transitions: Box<[usize]>,
}

pub struct DFA {
    pub states: Box<[DFAState]>,
    pub finals: BitVec,
}