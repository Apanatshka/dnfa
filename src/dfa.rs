use bit_vec::BitVec;
use std::fmt;
use std::mem;

use crate::automaton::{Automaton, Match};
use crate::nfa::{START, STUCK};

pub type Input = u8;
pub type StateNumber = usize;
pub type PatternNumber = usize;

pub struct DFAState {
    transitions: Box<[StateNumber]>,
    pattern_ends: Vec<PatternNumber>,
}

pub struct DFA {
    states: Box<[DFAState]>,
    finals: BitVec,
    dict: Vec<Vec<Input>>,
}

pub struct DDFA {
    states: Box<[DDFAState]>,
    dict: Vec<Vec<Input>>,
}

// Living dangerously: raw pointers baby
#[derive(Clone, PartialEq)]
pub struct DDFAState {
    transitions: Box<[*const DDFAState]>,
    pattern_ends: Vec<PatternNumber>,
    is_final: bool,
}

impl DFAState {
    pub fn new(transitions: Box<[StateNumber]>, pattern_ends: Vec<PatternNumber>) -> Self {
        DFAState {
            transitions,
            pattern_ends,
        }
    }
}

impl DFA {
    pub fn new(states: Box<[DFAState]>, finals: BitVec, dict: Vec<Vec<Input>>) -> Self {
        DFA {
            states,
            finals,
            dict,
        }
    }

    pub fn into_ddfa(self) -> Result<DDFA, ()> {
        let states_len = self.states.len();
        let mut states =
            vec![DDFAState::new(Box::new([]), Vec::new(), false); states_len].into_boxed_slice();

        let states_start: *mut DDFAState = (*states).as_mut_ptr();

        for (i, st) in self.states.iter().enumerate() {
            let mut v: Vec<*const DDFAState> = Vec::with_capacity(st.transitions.len());
            for &offset in st.transitions.iter() {
                if offset >= states_len {
                    return Err(());
                }
                unsafe {
                    v.push(states_start.offset(offset as isize) as *const DDFAState);
                }
            }
            states[i].transitions = v.into_boxed_slice();
            states[i].pattern_ends = self.states[i].pattern_ends.clone();
            states[i].is_final = self.finals[i];
        }
        Ok(DDFA::new(states, self.dict))
    }

    pub fn apply(&self, input: &[u8]) -> Vec<PatternNumber> {
        let mut cur_state = START;
        for &byte in input {
            cur_state = self.states[cur_state].transitions[byte as usize];
            if cur_state == STUCK {
                break;
            }
        }
        self.states[cur_state].pattern_ends.clone()
    }
}

impl Automaton<Input> for DFA {
    type State = StateNumber;

    fn start_state(&self) -> Self::State {
        START
    }

    fn stuck_state(&self) -> Self::State {
        STUCK
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
            patt_no,
            start: text_offset - self.dict[patt_no].len(),
            end: text_offset,
        }
    }
}

impl DDFAState {
    fn new(
        transitions: Box<[*const DDFAState]>,
        pattern_ends: Vec<PatternNumber>,
        is_final: bool,
    ) -> Self {
        DDFAState {
            transitions,
            pattern_ends,
            is_final,
        }
    }
}

impl DDFA {
    fn new(states: Box<[DDFAState]>, dict: Vec<Vec<Input>>) -> Self {
        DDFA { states, dict }
    }

    pub fn apply(&self, input: &[u8]) -> Vec<PatternNumber> {
        let mut cur_state: *const DDFAState = &self.states[START];
        let stuck = &self.states[STUCK];
        for &byte in input {
            cur_state = unsafe { (*cur_state).transitions[byte as usize] };
            if cur_state == stuck {
                break;
            }
        }
        unsafe { (*cur_state).pattern_ends.clone() }
    }
}

impl Automaton<Input> for DDFA {
    type State = *const DDFAState;

    fn start_state(&self) -> Self::State {
        &self.states[START]
    }

    fn stuck_state(&self) -> Self::State {
        &self.states[STUCK]
    }

    #[inline]
    fn next_state(&self, &state: &Self::State, &input: &Input) -> Self::State {
        unsafe { *(*state).transitions.get_unchecked(input as usize) }
    }

    #[inline]
    fn has_match(&self, &state: &Self::State, patt_no_offset: usize) -> bool {
        patt_no_offset < unsafe { (*state).pattern_ends.len() }
    }

    #[inline]
    fn get_match(&self, &state: &Self::State, patt_no_offset: usize, text_offset: usize) -> Match {
        let patt_no = unsafe { *(*state).pattern_ends.get_unchecked(patt_no_offset) };
        Match {
            patt_no,
            start: text_offset - self.dict[patt_no].len(),
            end: text_offset,
        }
    }
}

// The Debug::fmt implementation for DFA and DDFA are extremely similar. The only differences are in
//  computing the finality of a state and computing the index of a state in the states array.
// Therefore we share these with a macro:
macro_rules! debug_impl {
    ($struct_name:ident, $compute_finality:item, $compute_start:item, $compute_tr_no:item) => {
        impl fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                $compute_finality
                $compute_start
                $compute_tr_no
                let start = compute_start(&self);
                for (i, state) in (*self.states).into_iter().enumerate() {
                    if i == STUCK {
                        write!(f, "{} (stuck),\n", STUCK)?;
                        continue;
                    }
                    write!(f, "{}", i)?;
                    if i == START {
                        write!(f, " (start)")?;
                    }
                    if compute_finality(&self, state, i) {
                        write!(f, " (final)")?;
                    }
                    write!(f, ": {{")?;
                    if !state.transitions.is_empty() {
                        write!(f, "\n")?;
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
                            let tr_no = compute_tr_no(tr, start);
                            if c == last_c {
                                write!(f, "  {:?}: {:?},\n", c as u8 as char, tr_no)?;
                            } else {
                                write!(f, "  [{:?}-{:?}]: {:?},\n",
                                   last_c as u8 as char,
                                   (c as u8) as char,
                                   tr_no)?;
                            }
                            last_c = c2;
                        }
                    }
                    write!(f, "}},\n")?;
                }
                Ok(())
            }
        }
    }
}

debug_impl!(
    DFA,
    #[allow(unused_variables)]
    fn compute_finality(dfa: &DFA, state: &DFAState, i: usize) -> bool {
        dfa.finals[i]
    },
    #[allow(unused_variables)]
    fn compute_start(dfa: &DFA) -> () {
        ()
    },
    #[allow(unused_variables)]
    fn compute_tr_no(tr: &StateNumber, start: ()) -> &StateNumber {
        tr
    }
);

debug_impl!(
    DDFA,
    #[allow(unused_variables)]
    fn compute_finality(ddfa: &DDFA, state: &DDFAState, i: usize) -> bool {
        state.is_final
    },
    fn compute_start(ddfa: &DDFA) -> *const DDFAState {
        &ddfa.states[START] as *const DDFAState
    },
    fn compute_tr_no(tr: &*const DDFAState, start: *const DDFAState) -> usize {
        (*tr as usize - start as usize) / mem::size_of::<DDFAState>()
    }
);

#[cfg(test)]
mod tests {
    use crate::nfa::NFA;

    static BASIC_DICTIONARY: &'static [&'static str] = &["a", "ab", "bab", "bc", "bca", "c", "caa"];

    #[test]
    fn basic_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_prefixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(dfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!dfa.apply("bbc".as_bytes()).is_empty());
        assert!(dfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_postfixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(dfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(dfa.apply("bbc".as_bytes()).is_empty());
        assert!(!dfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_prefixes();
        nfa.ignore_postfixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(dfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!dfa.apply("bbc".as_bytes()).is_empty());
        assert!(!dfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_pre_postfixes_order() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(dfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!dfa.apply("bbc".as_bytes()).is_empty());
        assert!(!dfa.apply("abb".as_bytes()).is_empty());
    }

    use crate::automaton::Automaton;
    use std::iter;

    fn haystack_same(letter: char) -> String {
        iter::repeat(letter).take(10000).collect()
    }

    #[test]
    fn from_bench_basic_one_byte() {
        let needles = vec!["a"];
        let haystack = &haystack_same('z');

        let mut nfa = NFA::from_dictionary(needles);
        nfa.ignore_prefixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();

        assert!(dfa.find(haystack.as_bytes()).next().is_none());
    }

    static HAYSTACK_SHERLOCK: &'static str = include_str!("../benches/sherlock.txt");

    #[test]
    fn from_bench_sherlock_alt1() {
        let needles = vec!["Sherlock", "Street"];
        let count = 158;

        let haystack = HAYSTACK_SHERLOCK;

        let mut nfa = NFA::from_dictionary(needles);
        nfa.ignore_prefixes();
        let dfa = nfa.powerset_construction().into_dfa().unwrap();

        assert_eq!(count, dfa.find(haystack.as_bytes()).count());
    }
}
