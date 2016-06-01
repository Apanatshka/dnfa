extern crate bit_vec;

use self::bit_vec::BitVec;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

use super::dfa::{DFA, DFAState, DFA_STUCK, DFA_START};

pub type StateNumberSet = HashSet<usize>;

pub const NFA_START: usize = 0;

#[derive(Clone)]
pub struct NFAState {
    transitions: Vec<StateNumberSet>,
}

pub struct NFA {
    states: Vec<NFAState>,
    finals: BitVec,
}

impl NFA {
    pub fn from_dictionary(dict: &[&str]) -> Self {
        let num_states = dict.iter().fold(1, |sum, elem| sum + elem.len());
        let mut nfa = NFA {
            states: vec![NFAState { transitions: vec![HashSet::new(); 256] }; num_states],
            finals: BitVec::from_elem(num_states, false),
        };
        let mut nxt_state = NFA_START;
        for &string in dict.iter() {
            let mut cur_state = NFA_START;
            for byte in string.bytes() {
                if let Some(&state) = nfa.states[cur_state].transitions[byte as usize].get(&0) {
                    cur_state = state;
                } else {
                    nxt_state += 1;
                    nfa.states[cur_state].transitions[byte as usize].insert(nxt_state);
                    cur_state = nxt_state;
                }
            }
            nfa.finals.set(cur_state, true);
        }
        nfa
    }
    
    pub fn ignore_prefixes(&mut self) {
        for transition in &mut self.states[NFA_START].transitions {
            transition.insert(NFA_START);
        }
    }
    
    pub fn ignore_postfixes(&mut self) {
        for (fin,_) in self.finals.iter().enumerate().filter(|&(_,b)| b) {
            for mut transition in &mut self.states[fin].transitions {
                transition.insert(fin);
            }
        }
    }

    pub fn freeze(&self) -> Result<DFA, ()> {
        let mut states = Vec::with_capacity(self.states.len());
        for state in &self.states {
            states.push(try!(state.freeze()));
        }
        Ok(DFA::new(states.into_boxed_slice(), self.finals.clone()))
    }

    pub fn apply(&self, input: &[u8]) -> bool {
        let mut cur_states = HashSet::new();
        let mut nxt_states = HashSet::new();
        cur_states.insert(NFA_START);
        for &byte in input {
            for cur_state in cur_states {
                for &nxt_state in &self.states[cur_state].transitions[byte as usize] {
                    nxt_states.insert(nxt_state);
                }
            }
            cur_states = nxt_states;
            nxt_states = HashSet::new();
        }
        for state in cur_states {
            if self.finals[state] {
                return true;
            }
        }
        false
    }

    pub fn powerset_construction(&self) -> Self {
        let mut dnfa = NFA {
            states: vec![NFAState { transitions: vec![HashSet::new(); 256] }; 2],
            finals: BitVec::from_elem(2, false),
        };
        let mut states_map: HashMap<Vec<usize>, usize> = HashMap::new();
        let cur_states: HashSet<usize> = [NFA_START].into_iter().map(|x| *x).collect();

        dnfa.finals.set(DFA_START, self.finals.get(NFA_START).unwrap());

        states_map.insert(Vec::new(), DFA_STUCK);
        states_map.insert(cur_states.clone().into_iter().collect(), DFA_START);

        psc_rec_helper(self, &mut dnfa, &mut states_map, cur_states, DFA_START);
        dnfa
    }
}

impl fmt::Display for NFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, state) in (*self.states).into_iter().enumerate() {
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
                                      "  [{:?}-{:?}] -> {:?},",
                                      last_c as u8 as char,
                                      (c as u8) as char,
                                      tr));
                    }
                    last_c = c2;
                }
            }
            try!(write!(f, "]"));
            if i == NFA_START {
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

impl NFAState {
    fn freeze(&self) -> Result<DFAState, ()> {
        let mut transitions = Vec::with_capacity(self.transitions.len());
        for sns in &self.transitions {
            match sns.len() {
                0 => transitions.push(DFA_STUCK),
                1 => // Is there a better way to get this single element?
                    for &sn in sns {
                        transitions.push(sn);
                    },
                _ => return Err(()),
            }
        }
        Ok(DFAState::new(transitions.into_boxed_slice()))
    }
}

// Quick and dirty implementation, needs better thought out version
fn psc_rec_helper(nfa: &NFA,
                  dnfa: &mut NFA,
                  states_map: &mut HashMap<Vec<usize>, usize>,
                  cur_states: HashSet<usize>,
                  cur_num: usize) {
    for input in 0..255 {
        let mut nxt_states = HashSet::new();
        let mut fin = false;
        for cur_state in cur_states.clone() {
            let states = &nfa.states[cur_state].transitions[input];
            nxt_states = nxt_states.union(&mut states.clone()).map(|x| *x).collect();
            fin |= states.iter().map(|&st| nfa.finals.get(st).unwrap_or(false)).any(|x| x);
        }
        let nxt_states_vec = nxt_states.clone().into_iter().collect();
        match states_map.get(&nxt_states_vec) {
            Some(&nxt_num) => {
                dnfa.states[cur_num].transitions[input].insert(nxt_num);
            },
            None => {
                let nxt_num = dnfa.states.len();
                dnfa.states.push(NFAState { transitions: vec![HashSet::new(); 256] });
                dnfa.finals.push(fin);
                states_map.insert(nxt_states_vec, nxt_num);
                dnfa.states[cur_num].transitions[input].insert(nxt_num);
                if nxt_num != DFA_STUCK {
                    psc_rec_helper(nfa, dnfa, states_map, nxt_states, nxt_num);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {}
