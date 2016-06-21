extern crate bit_vec;

use self::bit_vec::BitVec;
use std::collections::BTreeMap;
use std::collections::Bound::{Included, Unbounded};
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::fmt;
use std::borrow::Borrow;

use automaton::{Automaton, Match};
use dfa::{DFA, DFAState};

pub const AUTO_START: usize = 0;
pub const AUTO_STUCK: usize = 1;

pub type StateNumberSet = BTreeSet<usize>;

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
                if let Some(nxts) = self.states[cur_state].transitions.get(&byte) {
                    nxt_states.extend(nxts);
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

        assert!(self.finals.get(AUTO_START).is_some());
        dnfa.finals.set(AUTO_START, self.finals.get(AUTO_START).unwrap());

        states_map.insert(Vec::new(), AUTO_STUCK);
        states_map.insert(cur_states.clone().into_iter().collect(), AUTO_START);

        let mut worklist = vec![(cur_states, AUTO_START)];
        while let Some((cur_states, cur_num)) = worklist.pop() {
            for &input in &dnfa.alphabet {
                let mut nxt_states = BTreeSet::new();
                let mut fin = false;
                for cur_state in cur_states.clone() {
                    assert!(self.states[cur_state].transitions.get(&input).is_some());
                    let states = self.states[cur_state].transitions.get(&input).unwrap();
                    nxt_states.extend(states);
                    for &st in states {
                        assert!(self.finals.get(st).is_some());
                    }
                    fin |= states.iter().map(|&st| self.finals.get(st).unwrap()).any(|x| x);
                }
                let nxt_states_vec = nxt_states.clone().into_iter().collect();
                let add_state = |states: &mut [NFAState], nxt_num| {
                    states[cur_num]
                        .transitions
                        .entry(input)
                        .or_insert_with(BTreeSet::new)
                        .insert(nxt_num);
                };
                match states_map.get(&nxt_states_vec) {
                    Some(&nxt_num) => {
                        if dnfa.states[cur_num]
                            .transitions
                            .get_lte(&input)
                            .map_or(true, |sns| !sns.contains(&nxt_num)) {
                            add_state(&mut dnfa.states, nxt_num);
                        }
                    }
                    None => {
                        let nxt_num = dnfa.states.len();
                        dnfa.states.push(NFAState { transitions: BTreeMap::new() });
                        dnfa.finals.push(fin);
                        states_map.insert(nxt_states_vec, nxt_num);
                        if nxt_num != AUTO_STUCK {
                            worklist.push((nxt_states, nxt_num));
                        }
                        add_state(&mut dnfa.states, nxt_num);
                    }
                }
            }
        }
        dnfa
    }
}

impl Automaton<u8> for NFA {
    type State = StateNumberSet;


    fn start_state() -> Self::State {
        [AUTO_START].iter().cloned().collect()
    }

    fn stuck_state() -> Self::State {
        [AUTO_STUCK].iter().cloned().collect()
    }

    #[inline]
    fn next_state(&self, states: &Self::State, input: &u8) -> Self::State {
        let mut nxt_states = BTreeSet::new();
        for &state in states {
            for &nxt_state in self.states[state].transitions.get(input).unwrap() {
                nxt_states.insert(nxt_state);
            }
        }
        nxt_states
    }

    fn has_match(&self, states: &Self::State, outi: usize) -> bool {
        for &state in states {
            if self.finals[state] {
                return true;
            }
        }
        false
    }

    fn get_match(&self, si: &Self::State, outi: usize, texti: usize) -> Match {
        unimplemented!()
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
mod tests {
    use super::*;

    static BASIC_DICTIONARY: &'static [&'static str] = &["a", "ab", "bab", "bc", "bca", "c", "caa"];

    #[test]
    fn basic() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(!nfa.apply("bbc".as_bytes()));
        assert!(!nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_prefixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(!nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_postfixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(!nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_prefixes();
        nfa.ignore_postfixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_ignore_pre_postfixes_order() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_powerset() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(!nfa.apply("bbc".as_bytes()));
        assert!(!nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_powerset_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_prefixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(!nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_powerset_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_postfixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(!nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_powerset_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_prefixes();
        nfa.ignore_postfixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }

    #[test]
    fn basic_powerset_ignore_pre_postfixes_order() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        for &word in BASIC_DICTIONARY {
            assert!(nfa.apply(word.as_bytes()));
        }
        assert!(nfa.apply("bbc".as_bytes()));
        assert!(nfa.apply("abb".as_bytes()));
    }
}
