extern crate bit_vec;

use self::bit_vec::BitVec;
use std::fmt;

pub const DFA_START: usize = 0; // Same as nfa::NFA_START!
pub const DFA_STUCK: usize = 1;

#[derive(Debug)]
pub struct DFAState {
    transitions: Box<[usize]>,
}

#[derive(Debug)]
pub struct DFA {
    states: Box<[DFAState]>,
    finals: BitVec,
}

pub struct DDFA {
    states: Box<[DDFAState]>,
}

// Living dangerously: raw pointers baby
#[derive(Clone, PartialEq)]
struct DDFAState {
    transitions: Box<[*const DDFAState]>,
    is_final: bool,
}

impl DFAState {
    pub fn new(transitions: Box<[usize]>) -> Self {
        DFAState { transitions: transitions }
    }
}

impl DFA {
    pub fn new(states: Box<[DFAState]>, finals: BitVec) -> Self {
        DFA {
            states: states,
            finals: finals,
        }
    }

    pub fn into_ddfa(self) -> Result<DDFA, ()> {
        let len = self.states.len();
        let mut states = vec![DDFAState { transitions: Box::new([]), is_final: false }; len]
            .into_boxed_slice();

        let states_start: *mut DDFAState = (*states).as_mut_ptr();

        for (i, ref st) in self.states.iter().enumerate() {
            let mut v: Vec<*const DDFAState> = Vec::with_capacity(st.transitions.len());
            for &offset in st.transitions.iter() {
                if offset >= len {
                    return Err(());
                }
                unsafe {
                    v.push(states_start.offset(offset as isize) as *const DDFAState);
                }
            }
            states[i].transitions = v.into_boxed_slice();
            states[i].is_final = self.finals[i];
        }
        Ok(DDFA { states: states })
    }

    pub fn apply(&self, input: &[u8]) -> bool {
        let mut cur_state = DFA_START;
        for &byte in input {
            cur_state = self.states[cur_state].transitions[byte as usize];
            if cur_state == DFA_STUCK {
                break;
            }
        }
        self.finals[cur_state]
    }
}

impl fmt::Display for DFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, state) in (*self.states).into_iter().enumerate() {
            if i == DFA_STUCK {
                try!(writeln!(f, "{} -- stuck state,", DFA_STUCK));
                continue;
            }
            try!(write!(f, "{} -> [", i));
            if !state.transitions.is_empty() {
                try!(writeln!(f, ""));
            }
            let mut last_c = 0;
            let mut iter = (*state.transitions)
                .into_iter()
                .enumerate()
                .peekable();
            while let Some((c, tr)) = iter.next() {
                if let Some(&(c2, tr2)) = iter.peek() {
                    if tr == tr2 {
                        continue;
                    }
                    if c == last_c {
                        try!(writeln!(f, "  {:?} -> {:?},", c as u8 as char, tr));
                    } else {
                        try!(writeln!(f,
                                      "  [{:?},{:?}] -> {:?},",
                                      last_c as u8 as char,
                                      (c as u8) as char,
                                      tr));
                    }
                    last_c = c2;
                }
            }
            try!(write!(f, "]"));
            if i == DFA_START {
                try!(write!(f, " -- start state"));
            }
            if self.finals[i] {
                try!(write!(f, " -- final state"));
            }
            try!(writeln!(f, ","));
        }
        Ok(())
    }
}

impl DDFA {
    pub fn apply(&self, input: &[u8]) -> bool {
        let mut cur_state: *const DDFAState = &self.states[DFA_START];
        let stuck = &self.states[DFA_STUCK];
        for &byte in input {
            cur_state = unsafe { cur_state.offset(byte as isize) };
            if cur_state == stuck {
                break;
            }
        }
        unsafe { (*cur_state).is_final }
    }
}

#[cfg(test)]
mod tests {}
