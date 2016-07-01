extern crate bit_vec;

use self::bit_vec::BitVec;
use std::mem;
use std::fmt;

use nfa::{AUTO_START, AUTO_STUCK};
use automaton::{Automaton, Match};

pub type Input = u8;
pub type StateNumber = usize;
pub type PatternNumber = usize;

#[derive(Default)]
pub struct DFAState {
    transitions: Box<[StateNumber]>,
    pattern_ends: Box<[PatternNumber]>,
}

#[derive(Default)]
pub struct DFA {
    states: Box<[DFAState]>,
    finals: BitVec,
    dict: Vec<Vec<Input>>,
}

#[derive(Default)]
pub struct DDFA {
    states: Box<[DDFAState]>,
    dict: Vec<Vec<Input>>,
}

// Living dangerously: raw pointers baby
#[derive(Clone, PartialEq)]
struct DDFAState {
    transitions: Box<[*const DDFAState]>,
    is_final: bool,
}

impl DFAState {
    pub fn new(transitions: Box<[StateNumber]>, pattern_ends: Box<[PatternNumber]>) -> Self {
        DFAState {
            transitions: transitions,
            pattern_ends: pattern_ends,
        }
    }
}

impl DFA {
    pub fn new(states: Box<[DFAState]>, finals: BitVec, dict: Vec<Vec<Input>>) -> Self {
        DFA {
            states: states,
            finals: finals,
            dict: dict,
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
        Ok(DDFA {
            states: states,
            dict: self.dict,
        })
    }

    pub fn apply(&self, input: &[u8]) -> bool {
        let mut cur_state = AUTO_START;
        for &byte in input {
            cur_state = self.states[cur_state].transitions[byte as usize];
            if cur_state == AUTO_STUCK {
                break;
            }
        }
        self.finals[cur_state]
    }
}

impl Automaton<Input> for DFA {
    type State = StateNumber;


    fn start_state() -> Self::State {
        AUTO_START
    }

    fn stuck_state() -> Self::State {
        AUTO_STUCK
    }

    #[inline]
    fn next_state(&self, &state: &Self::State, &input: &Input) -> Self::State {
        self.states[state].transitions[input as usize]
    }

    #[inline]
    fn has_match(&self, &state: &Self::State, patt_no_offset: usize) -> bool {
        patt_no_offset < self.states[state].pattern_ends.len()
    }

    #[inline]
    fn get_match(&self, &state: &Self::State, patt_no_offset: usize, text_offset: usize) -> Match {
        let patt_no = self.states[state].pattern_ends[patt_no_offset];
        Match {
            patt_no: patt_no,
            start: text_offset - self.dict[patt_no].len(),
            end: text_offset,
        }
    }
}

impl fmt::Debug for DFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! w {
            ($($tt:tt)*) => { try!(write!(f, $($tt)*)) }
        }
        for (i, state) in (*self.states).into_iter().enumerate() {
            if i == AUTO_STUCK {
                w!("{} (stuck),\n", AUTO_STUCK);
                continue;
            }
            w!("{}", i);
            if i == AUTO_START {
                w!(" (start)");
            }
            if self.finals[i] {
                w!(" (final)");
            }
            w!(": {{");
            if !state.transitions.is_empty() {
                w!("\n");
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
                        w!("  {:?}: {:?},\n", c as u8 as char, tr);
                    } else {
                        w!("  [{:?}-{:?}]: {:?},\n",
                           last_c as u8 as char,
                           (c as u8) as char,
                           tr);
                    }
                    last_c = c2;
                }
            }
            w!("}},\n");
        }
        Ok(())
    }
}

impl DDFA {
    pub fn apply(&self, input: &[u8]) -> bool {
        let mut cur_state: *const DDFAState = &self.states[AUTO_START];
        let stuck = &self.states[AUTO_STUCK];
        for &byte in input {
            cur_state = unsafe { (*cur_state).transitions[byte as usize] };
            if cur_state == stuck {
                break;
            }
        }
        unsafe { (*cur_state).is_final }
    }
}

impl fmt::Debug for DDFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! w {
            ($($tt:tt)*) => { try!(write!(f, $($tt)*)) }
        }
        let start = &self.states[0] as *const DDFAState;
        for (i, state) in (*self.states).into_iter().enumerate() {
            if i == AUTO_STUCK {
                w!("{} (stuck),\n", AUTO_STUCK);
                continue;
            }
            w!("{}", i);
            if i == AUTO_START {
                w!(" (start)");
            }
            if state.is_final {
                w!(" (final)");
            }
            w!(": {{");
            if !state.transitions.is_empty() {
                w!("\n");
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
                    let tr_no = (*tr as usize - start as usize) / mem::size_of::<DDFAState>();
                    if c == last_c {
                        w!("  {:?}: {:?},\n", c as u8 as char, tr_no);
                    } else {
                        w!("  [{:?}-{:?}]: {:?},\n",
                           last_c as u8 as char,
                           (c as u8) as char,
                           tr_no);
                    }
                    last_c = c2;
                }
            }
            w!("}},\n");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
