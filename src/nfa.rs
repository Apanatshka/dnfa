extern crate bit_vec;

use self::bit_vec::BitVec;
use std::collections::HashMap;

use super::dfa::{DFA, DFAState, DFA_STUCK, DFA_START};

pub type StateNumberSet = Vec<usize>;

pub const NFA_START: usize = 0;

#[derive(Clone)]
pub struct NFAState {
    pub transitions: Vec<StateNumberSet>,
}

pub struct NFA {
    pub states: Vec<NFAState>,
    pub finals: BitVec,
}

impl NFA {
    pub fn from_dictionary(dict: &[&str]) -> Self {
        let num_states = dict.iter().fold(1, |sum, elem| sum + elem.len());
        let mut nfa = NFA {
            states: vec![NFAState { transitions: vec![Vec::new(); 256] }; num_states],
            finals: BitVec::from_elem(num_states, false),
        };
        nfa.states[NFA_START].transitions = vec![vec![NFA_START]; 256];
        let mut nxt_state = NFA_START;
        for &string in dict.iter() {
            let mut cur_state = NFA_START;
            for byte in string.bytes() {
                nxt_state += 1;
                nfa.states[cur_state].transitions[byte as usize].push(nxt_state);
                cur_state = nxt_state;
            }
            nfa.finals.set(cur_state, true);
        }
        nfa
    }

    pub fn freeze(self) -> Result<DFA, ()> {
        let mut states = Vec::with_capacity(self.states.len());
        for state in self.states {
            states.push(try!(state.freeze_state()));
        }
        Ok(DFA {
            states: states.into_boxed_slice(), 
            finals: self.finals.clone(),
        })
    }
}

impl NFAState {
    fn freeze_state(self) -> Result<DFAState, ()> {
        let mut transitions = Vec::with_capacity(self.transitions.len());
        for sns in self.transitions {
            if sns.len() == 0 {
                transitions.push(DFA_STUCK);
            } else if sns.len() == 1 {
                transitions.push(sns[0]);
            } else {
                return Err(());
            }
        }
        Ok(DFAState {
            transitions: transitions.into_boxed_slice()
        })
    }
}

pub fn powerset_construction(nfa: NFA) -> NFA {
    let mut dnfa = NFA {
        states: vec![NFAState { transitions: vec![Vec::new(); 256] }; 2],
        finals: BitVec::from_elem(2, false),
    };
    let mut states_map = HashMap::new();
    let cur_states = vec![NFA_START];
    
    dnfa.finals.set(DFA_START, nfa.finals.get(NFA_START).unwrap());
    
    states_map.insert(Vec::new(), DFA_STUCK);
    states_map.insert(cur_states.clone(), DFA_START);
    
    psc_rec_helper(&nfa, &mut dnfa, &mut states_map, cur_states, DFA_START);
    dnfa
}

fn psc_rec_helper(nfa: &NFA, dnfa: &mut NFA, 
        states_map: &mut HashMap<StateNumberSet, usize>, 
        cur_states: StateNumberSet, cur_num: usize) {
    for input in 0..255 {
        let mut nxt_states = Vec::new();
        for cur_state in cur_states.clone() {
            let ref states = nfa.states[cur_state].transitions[input];
            nxt_states.append(&mut states.clone());
        }
        let fin = nxt_states.iter().map(|&st| nfa.finals.get(st).unwrap_or(false))
                    .any(|x| x);
        match states_map.get(&nxt_states) {
            Some(&nxt_num) => dnfa.states[cur_num].transitions[input].push(nxt_num),
            None => {
                let nxt_num = dnfa.states.len();
                dnfa.states.push(NFAState { transitions: vec![Vec::new(); 256] });
                dnfa.finals.push(fin);
                states_map.insert(nxt_states.clone(), nxt_num);
                dnfa.states[cur_num].transitions[input].push(nxt_num);
                if nxt_num != DFA_STUCK {
                    psc_rec_helper(nfa, dnfa, states_map, nxt_states, nxt_num);
                }
            },
        }
    }
}