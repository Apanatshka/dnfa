extern crate bit_vec;

use self::bit_vec::BitVec;
use std::collections::BTreeMap;
use std::collections::Bound::{Included, Unbounded};
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::fmt;
use std::borrow::Borrow;

use automaton::{AUTO_START, AUTO_STUCK};
use dfa::{DFA, DFAState};

type StateNumberSet = BTreeSet<usize>;

#[derive(Clone)]
struct NFAState {
    transitions: BTreeMap<u8, StateNumberSet>,
}

pub struct NFA {
    alphabet: Vec<u8>,
    states: Vec<NFAState>,
    finals: BitVec,
}

impl NFA {
    pub fn from_dictionary(dict: &[&str]) -> Self {
        let num_states = dict.iter().fold(1, |sum, elem| sum + elem.len());
        let transitions: BTreeMap<u8, StateNumberSet> =
            vec![BTreeSet::new(); 256].into_iter().enumerate().map(|(k, v)| (k as u8, v)).collect();
        let mut nfa = NFA {
            alphabet: Vec::new(),
            states: vec![NFAState { transitions: transitions.clone() }; num_states],
            finals: BitVec::from_elem(num_states, false),
        };
        let mut nxt_state = AUTO_START;
        let mut alphabet = BTreeSet::new();
        for &string in dict.iter() {
            let mut cur_state = AUTO_START;
            for byte in string.bytes() {
                alphabet.insert(byte);
                if let Some(&state) = nfa.states[cur_state]
                    .transitions
                    .get(&byte)
                    .unwrap()
                    .get(&0) {
                    cur_state = state;
                } else {
                    nxt_state += 1;
                    nfa.states[cur_state].transitions.get_mut(&byte).unwrap().insert(nxt_state);
                    cur_state = nxt_state;
                }
            }
            nfa.finals.set(cur_state, true);
        }
        nfa.alphabet = alphabet.into_iter().collect();
        nfa
    }

    pub fn ignore_prefixes(&mut self) {
        for (_, transition) in &mut self.states[AUTO_START].transitions {
            transition.insert(AUTO_START);
        }
    }

    pub fn ignore_postfixes(&mut self) {
        for (fin, _) in self.finals.iter().enumerate().filter(|&(_, b)| b) {
            for (_, transition) in &mut self.states[fin].transitions {
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
        let mut cur_states = BTreeSet::new();
        let mut nxt_states = BTreeSet::new();
        cur_states.insert(AUTO_START);
        for &byte in input {
            for cur_state in cur_states {
                for &nxt_state in self.states[cur_state].transitions.get(&byte).unwrap() {
                    nxt_states.insert(nxt_state);
                }
            }
            cur_states = nxt_states;
            nxt_states = BTreeSet::new();
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
            alphabet: self.alphabet.clone(),
            states: vec![NFAState { transitions: BTreeMap::new() }; 2],
            finals: BitVec::from_elem(2, false),
        };
        let mut states_map: HashMap<Vec<usize>, usize> = HashMap::new();
        let cur_states: BTreeSet<usize> = [AUTO_START].into_iter().cloned().collect();

        dnfa.finals.set(AUTO_START, self.finals.get(AUTO_START).unwrap());

        states_map.insert(Vec::new(), AUTO_STUCK);
        states_map.insert(cur_states.clone().into_iter().collect(), AUTO_START);

        let mut worklist = vec![(cur_states, AUTO_START)];
        while let Some((cur_states, cur_num)) = worklist.pop() {
            for &input in &dnfa.alphabet {
                let mut nxt_states = BTreeSet::new();
                let mut fin = false;
                for cur_state in cur_states.clone() {
                    let states = self.states[cur_state].transitions.get(&input).unwrap();
                    nxt_states.extend(states);
                    fin |= states.iter().map(|&st| self.finals.get(st).unwrap()).any(|x| x);
                }
                let nxt_states_vec = nxt_states.clone().into_iter().collect();
                match states_map.get(&nxt_states_vec) {
                    Some(&nxt_num) => {
                        if dnfa.states[cur_num]
                            .transitions
                            .get_lte(&input)
                            .map(|sns| !sns.contains(&nxt_num))
                            .unwrap_or(true) {
                            dnfa.states[cur_num]
                                .transitions
                                .entry(input)
                                .or_insert(BTreeSet::new())
                                .insert(nxt_num);
                        }
                    }
                    None => {
                        let nxt_num = dnfa.states.len();
                        dnfa.states.push(NFAState { transitions: BTreeMap::new() });
                        dnfa.finals.push(fin);
                        states_map.insert(nxt_states_vec, nxt_num);
                        dnfa.states[cur_num]
                            .transitions
                            .entry(input)
                            .or_insert(BTreeSet::new())
                            .insert(nxt_num);
                        if nxt_num != AUTO_STUCK {
                            worklist.push((nxt_states, nxt_num));
                        }
                    }
                }
            }
        }
        dnfa
    }
}

trait BTreeMapExt<K, V> {
    fn get_lte<Q>(&self, key: &Q) -> Option<&V>
        where Q: Ord,
              K: Borrow<Q>;
}

impl<K: Ord, V> BTreeMapExt<K, V> for BTreeMap<K, V> {
    fn get_lte<Q: ?Sized>(&self, key: &Q) -> Option<&V>
        where Q: Ord,
              K: Borrow<Q>
    {
        self.range(Unbounded, Included(key)).last().map(|x| x.1)
    }
}

impl fmt::Display for NFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, state) in (*self.states).into_iter().enumerate() {
            try!(write!(f, "{} -> [", i));
            if !state.transitions.is_empty() {
                try!(writeln!(f, ""));
            }
            let mut iter = state.transitions.iter();
            let mut c = 0;
            let mut tr: &BTreeSet<usize> = &[AUTO_STUCK].iter().cloned().collect();
            loop {
                if let Some((&c2, ref tr2)) = iter.next() {
                    if c == c2 - 1 {
                        try!(writeln!(f, "  {:?} -> {:?},", c as u8 as char, tr));
                    } else {
                        try!(writeln!(f,
                                      "  [{:?}-{:?}] -> {:?},",
                                      c as u8 as char,
                                      c2 as u8 as char,
                                      tr));
                    }
                    c = c2;
                    tr = tr2;
                } else {
                    if c == 255 {
                        try!(writeln!(f, "  {:?} -> {:?},", c as u8 as char, tr));
                    } else {
                        try!(writeln!(f,
                                      "  [{:?}-{:?}] -> {:?},",
                                      c as u8 as char,
                                      255 as char,
                                      tr));
                    }
                    break;
                }
            }
            try!(write!(f, "]"));
            if i == AUTO_START {
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
        let mut transitions = vec![AUTO_STUCK; 256];
        for (&i, ref sns) in &self.transitions {
            match sns.len() {
                1 => // Is there a better way to get this single element?
                    for &sn in *sns {
                        transitions[i as usize] = sn;
                    },
                _ => return Err(()),
            }
        }
        Ok(DFAState::new(transitions.into_boxed_slice()))
    }
}

#[cfg(test)]
mod tests {}
