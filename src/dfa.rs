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

pub struct DDFA {
    pub states: Box<[DDFAState]>,
    pub finals: BitVec,
}

// Living dangerously: raw pointers baby
#[derive(Clone)]
pub struct DDFAState {
    transitions: Box<[*const DDFAState]>,
    pub is_final: bool,
}

impl DFA {
    pub fn to_ddfa(self) -> Result<DDFA, ()> {
        let len = self.states.len();
        let mut states = vec![DDFAState { transitions: Box::new([]), is_final: false }; len]
            .into_boxed_slice();
        
        let states_start: *mut DDFAState = (*states).as_mut_ptr();

        for (i, ref st) in self.states.iter().enumerate() {
            let mut v: Vec<*const DDFAState> = Vec::with_capacity(st.transitions.len());
            for &offset in st.transitions.iter() {
                if offset >= len {
                    return Err(())
                }
                unsafe {
                    v.push(states_start.offset(offset as isize) as *const DDFAState);
                }
            }
            states[i].transitions = v.into_boxed_slice();
            states[i].is_final = self.finals[i];
        }
        Ok(DDFA { states: states, finals: self.finals })
    }
}

impl DDFAState {
    pub fn get(&self, offset: usize) -> DDFAState {
        unsafe { (*self.transitions[offset]).clone() }
    }
}
