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
pub type PatternNumber = usize;

#[derive(Clone, Default)]
struct NFAState {
    transitions: BTreeMap<u8, StateNumberSet>,
    pattern_ends: Vec<PatternNumber>,
}

#[derive(Default)]
pub struct NFA {
    alphabet: Vec<u8>,
    states: Vec<NFAState>,
    dict: Vec<String>,
}

impl NFA {
    pub fn new() -> Self {
        NFA {
            alphabet: Vec::new(),
            states: Vec::new(),
            dict: Vec::new(),
        }
    }

    pub fn with_alphabet(alphabet: Vec<u8>) -> Self {
        NFA {
            alphabet: alphabet,
            states: Vec::new(),
            dict: Vec::new(),
        }
    }

    pub fn from_dictionary(dict: Vec<&str>) -> Self {
        let num_states = dict.iter().fold(1, |sum, elem| sum + elem.len());
        let mut nfa = NFA {
            alphabet: Vec::new(),
            states: vec![NFAState::full(); num_states],
            dict: dict.iter().cloned().map(|s| s.to_owned()).collect(),
        };
        let mut nxt_state = AUTO_START;
        let mut alphabet = BTreeSet::new();
        for (pattern_no, &string) in dict.iter().enumerate() {
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
            nfa.states[cur_state].pattern_ends.push(pattern_no);
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
        let finals = self.states.iter_mut().enumerate().filter(|&(_, ref st)| st.is_final());
        for (fin, state) in finals {
            for (_, transition) in &mut state.transitions {
                transition.insert(fin);
            }
        }
    }

    pub fn freeze(&self) -> Result<DFA, ()> {
        let mut states = Vec::with_capacity(self.states.len());
        for state in &self.states {
            states.push(try!(state.freeze()));
        }
        let finals = BitVec::from_fn(self.states.len(), |i| self.states[i].is_final());
        Ok(DFA::new(states.into_boxed_slice(), finals))
    }

    pub fn apply(&self, input: &[u8]) -> Vec<PatternNumber> {
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
        cur_states.iter().flat_map(|&state| self.states[state].pattern_ends.clone()).collect()
    }

    pub fn powerset_construction(&self) -> Self {
        let mut dnfa = NFA {
            alphabet: self.alphabet.clone(),
            states: vec![NFAState::new(); 2],
            dict: self.dict.clone(),
        };
        let mut states_map: HashMap<Vec<usize>, usize> = HashMap::new();
        let cur_states: BTreeSet<usize> = [AUTO_START].into_iter().cloned().collect();

        dnfa.states[AUTO_START].pattern_ends = self.states[AUTO_START].pattern_ends.clone();

        states_map.insert(Vec::new(), AUTO_STUCK);
        states_map.insert(cur_states.clone().into_iter().collect(), AUTO_START);

        let mut worklist = vec![(cur_states, AUTO_START)];
        while let Some((cur_states, cur_num)) = worklist.pop() {
            for &input in &dnfa.alphabet {
                let mut nxt_states = BTreeSet::new();
                let mut fin = BTreeSet::new();
                for cur_state in cur_states.clone() {
                    assert!(self.states[cur_state].transitions.get(&input).is_some());
                    let states = self.states[cur_state].transitions.get(&input).unwrap();
                    nxt_states.extend(states);
                    for &st in states {
                        fin.extend(self.states[st].pattern_ends.clone());
                    }
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
                        let mut new_state = NFAState::new();
                        new_state.pattern_ends = fin.into_iter().collect();
                        dnfa.states.push(new_state);
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

    #[inline]
    fn has_match(&self, states: &Self::State, patt_no_offset: usize) -> bool {
        for &state in states {
            if patt_no_offset < self.states[state].pattern_ends.len() {
                return true;
            }
        }
        false
    }

    #[inline]
    fn get_match(&self, states: &Self::State, patt_no_offset: usize, text_offset: usize) -> Match {
        for &state in states {
            if let Some(&patt_no) = self.states[state].pattern_ends.get(patt_no_offset) {
                return Match {
                    patt_no: patt_no,
                    start: text_offset - self.dict[patt_no].len(),
                    end: text_offset,
                };
            }
        }
        panic!("There is no match of this pattern!");
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
            if self.states[i].is_final() {
                try!(write!(f, " -- final state"));
            }
            try!(writeln!(f, ","));
        }
        Ok(())
    }
}

impl NFAState {
    fn new() -> Self {
        NFAState {
            transitions: BTreeMap::new(),
            pattern_ends: Vec::new(),
        }
    }

    fn full() -> Self {
        let transitions: BTreeMap<u8, StateNumberSet> =
            vec![BTreeSet::new(); 256].into_iter().enumerate().map(|(k, v)| (k as u8, v)).collect();
        NFAState {
            transitions: transitions,
            pattern_ends: Vec::new(),
        }
    }

    fn is_final(&self) -> bool {
        !self.pattern_ends.is_empty()
    }

    fn freeze(&self) -> Result<DFAState, ()> {
        let mut transitions = vec![AUTO_STUCK; 256];
        for (&i, ref sns) in &self.transitions {
            if sns.len() != 1 {
                return Err(());
            }
            for &sn in *sns {
                transitions[i as usize] = sn;
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
        let nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect());
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect());
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect());
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect());
        nfa.ignore_prefixes();
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_pre_postfixes_order() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect());
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset() {
        let nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect())
            .powerset_construction();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect())
            .powerset_construction();
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect())
            .powerset_construction();
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect())
            .powerset_construction();
        nfa.ignore_prefixes();
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_pre_postfixes_order() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY.iter().cloned().collect())
            .powerset_construction();
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }
}
