extern crate bit_vec;

use self::bit_vec::BitVec;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::fmt;

use crate::automaton::{Automaton, Match};
use crate::dfa::{DFA, DFAState};

pub const AUTO_START: usize = 0;
pub const AUTO_STUCK: usize = 1;

pub type Input = u8;
pub type StateNumber = usize;
pub type PatternNumber = usize;
pub type Depth = usize;

#[derive(Clone, Default)]
struct NFAState {
    transitions: BTreeMap<Input, BTreeSet<StateNumber>>,
    pattern_ends: Vec<PatternNumber>,
}

#[derive(Default)]
pub struct NFA {
    alphabet: Vec<Input>,
    states: Vec<NFAState>,
    dict: Vec<Vec<Input>>,
    depth_map: BTreeMap<Depth, BTreeSet<StateNumber>>,
}

impl NFA {
    pub fn new() -> Self {
        NFA {
            alphabet: Vec::new(),
            states: Vec::new(),
            dict: Vec::new(),
            depth_map: BTreeMap::new(),
        }
    }

    pub fn from_dictionary<P, I>(dict: I) -> Self
        where P: AsRef<[u8]>,
              I: IntoIterator<Item = P> + Clone
    {
        let mut nfa = NFA {
            alphabet: Vec::new(),
            states: Vec::new(),
            dict: dict.clone().into_iter().map(|p| p.as_ref().to_vec()).collect(),
            depth_map: BTreeMap::new(),
        };
        // the start and stuck states
        nfa.states.push(NFAState::new());
        nfa.states.push(NFAState::new());

        // collect the alphabet from the patterns while we're looping through them anyway
        let mut alphabet = BTreeSet::new();
        for (pattern_no, bytes) in dict.into_iter().enumerate() {
            let mut cur_state = AUTO_START;
            for &byte in bytes.as_ref() {
                alphabet.insert(byte);
                // If there is a transition on this byte from the cur_state
                //  just go there. (We can be sure there will be only one at this point)
                if let Some(&state) = nfa.states[cur_state]
                    .transitions
                    .get(&byte)
                    .map_or(None, |x| x.iter().next()) {
                    cur_state = state;
                }
                // Otherwise add a new transition, and add the corresponding state
                else {
                    let nxt_state = nfa.states.len();
                    nfa.states.push(NFAState::new());
                    nfa.states[cur_state]
                        .transitions
                        .entry(byte)
                        .or_insert_with(BTreeSet::new)
                        .insert(nxt_state);
                    cur_state = nxt_state;
                }
            }
            nfa.states[cur_state].pattern_ends.push(pattern_no);
        }

        nfa.alphabet = alphabet.into_iter().collect();
        nfa
    }

    pub fn add_depth_map(&mut self) {
        if !self.depth_map.is_empty() {
            return;
        }
        let states_len = self.states.len();

        let mut depth = 0;

        let mut visited = vec![false; states_len];
        let mut cur_nodes = BTreeSet::new();
        let mut nxt_nodes = BTreeSet::new();

        cur_nodes.insert(AUTO_START);

        while !cur_nodes.is_empty() {
            for &node in &cur_nodes {
                visited[node] = true;
                for &byte in &self.alphabet {
                    if let Some(states) = self.states[node].transitions.get(&byte) {
                        nxt_nodes.extend(states.into_iter()
                            .filter(|&&n| !visited[n])
                            .cloned()
                            .collect::<BTreeSet<StateNumber>>());
                    }
                }
            }
            self.depth_map.insert(depth, cur_nodes);
            cur_nodes = nxt_nodes;
            nxt_nodes = BTreeSet::new();
            depth += 1;
        }
    }

    pub fn ignore_prefixes(&mut self) {
        self.alphabet = (0..=255).collect();
        for &byte in &self.alphabet {
            self.states[AUTO_START]
                .transitions
                .entry(byte)
                .or_insert_with(BTreeSet::new)
                .insert(AUTO_START);
        }
    }

    pub fn ignore_postfixes(&mut self) {
        self.alphabet = (0..=255).collect();
        let finals = self.states.iter_mut().enumerate().filter(|&(_, ref st)| st.is_final());
        for (fin, state) in finals {
            for &byte in &self.alphabet {
                state.transitions.entry(byte).or_insert_with(BTreeSet::new).insert(fin);
            }
        }
    }

    pub fn into_dfa(self) -> Result<DFA, ()> {
        let finals = BitVec::from_fn(self.states.len(), |i| self.states[i].is_final());
        let mut states = Vec::with_capacity(self.states.len());
        for state in self.states {
            states.push(state.into_dfa()?);
        }
        Ok(DFA::new(states.into_boxed_slice(), finals, self.dict))
    }

    pub fn apply(&self, input: &[Input]) -> Vec<PatternNumber> {
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

    // Changed from a recursive algorithm to a worklist (stack) algorithm
    // i.e., it keeps its own stack instead of using the function stack
    pub fn powerset_construction(&self) -> Self {
        // dnfa setup, two states: start and stuck, already in there
        let mut dnfa = NFA {
            alphabet: self.alphabet.clone(),
            states: vec![NFAState::new(); 2],
            dict: self.dict.clone(),
            depth_map: BTreeMap::new(),
        };
        // Maps sets of state-numbers from the NFA, to state-numbers of the DNFA
        let mut states_map: HashMap<Vec<StateNumber>, StateNumber> = HashMap::new();
        // Set of states that the NFA is in
        let cur_states: BTreeSet<StateNumber> = [AUTO_START].into_iter().cloned().collect();

        dnfa.states[AUTO_START].pattern_ends = self.states[AUTO_START].pattern_ends.clone();

        // While executing an NFA, no states means we're stuck,
        states_map.insert(Vec::new(), AUTO_STUCK);
        // stuck state only means we're stuck,
        states_map.insert(vec![AUTO_STUCK], AUTO_STUCK);
        // start state only means we're at the start.
        states_map.insert(vec![AUTO_START], AUTO_START);

        // The "recursive" part. We start in only the start state.
        // For every item (nfa-state-set, dfa-state), we go over every symbol in the alphabet.
        // For every symbol we discover the new nfa-state-set `nxt_states` by following the nfa
        //   transitions.
        // The new state-set is given a dfa-state `new_state` and put on the `worklist` if we
        //  haven't seen it yet.
        // We can check if we've seen it yet with the states_map.
        // When we add a new item to the worklist we add a transition to the dfa from the current
        //  dfa-state to the new one, labeled with the current symbol of the alphabet.
        let mut worklist = vec![(cur_states, AUTO_START)];
        while let Some((cur_states, cur_num)) = worklist.pop() {
            for &input in &dnfa.alphabet {
                let mut nxt_states = BTreeSet::new();
                let mut fin = BTreeSet::new();
                for &cur_state in &cur_states {
                    if let Some(states) = self.states[cur_state].transitions.get(&input) {
                        nxt_states.extend(states);
                        for &st in states {
                            fin.extend(self.states[st].pattern_ends.clone());
                        }
                    }
                }
                let nxt_states_vec: Vec<StateNumber> = nxt_states.clone().into_iter().collect();

                let nxt_num = {
                    let dnfa_states = &mut dnfa.states;
                    states_map.get(&nxt_states_vec).cloned().unwrap_or_else(|| {
                        let nxt_num = dnfa_states.len();
                        let mut new_state = NFAState::new();
                        new_state.pattern_ends = fin.into_iter().collect();
                        dnfa_states.push(new_state);
                        states_map.insert(nxt_states_vec, nxt_num);
                        if nxt_num != AUTO_STUCK {
                            worklist.push((nxt_states, nxt_num));
                        }
                        nxt_num
                    })
                };

                dnfa.states[cur_num]
                    .transitions
                    .entry(input)
                    .or_insert_with(BTreeSet::new)
                    .insert(nxt_num);
            }
        }
        dnfa
    }

    #[doc(hidden)]
    pub fn dot(&self, options: DotOptions) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        macro_rules! w {
            ($($tt:tt)*) => { {write!(out, $($tt)*)}.unwrap() }
        }

        let dict_comma_string = self.dict
            .clone()
            .into_iter()
            .map(|x| String::from_utf8_lossy(x.as_ref()).into_owned())
            .collect::<Vec<String>>()
            .join(", ");

        w!(r#"
digraph automaton {{
    label=<<FONT POINT-SIZE="20">{}</FONT>>;
    labelloc="l";
    labeljust="l";
    rankdir="LR";
    start [shape="none", label="", width=0];
    start -> 0;
"#,
           dict_comma_string);

        let mut original_edges = BTreeSet::new();

        for bytes in &self.dict {
            let mut cur_state = AUTO_START;
            for &byte in bytes {
                if let Some(nxt_states) = self.states[cur_state].transitions.get(&byte) {
                    if let Some(&nxt_state) = nxt_states.iter().next() {
                        original_edges.insert((cur_state, nxt_state));
                        cur_state = nxt_state;
                    }
                }
            }
        }

        for (from, state) in (*self.states).into_iter().enumerate() {
            if options.suppress_stuck_state && from == AUTO_STUCK {
                continue;
            }
            w!("    {}", from);
            if from == AUTO_STUCK {
                w!(r#" [label="⊥"]"#);
            }
            if self.states[from].is_final() {
                w!(" [peripheries=2]");
            }
            w!(";\n");
            let flipped_transitions = flip_multimap(state.transitions.clone());
            for (to, bytes) in flipped_transitions {
                if options.suppress_stuck_state && to == AUTO_STUCK {
                    continue;
                }
                w!("    {} -> {}", from, to);
                if options.bold_dict_edges && original_edges.contains(&(from, to)) {
                    w!(" [style=bold]");
                }
                w!(" [label=\"{}\"];\n",
                   implode_ranges(bytes)
                       .join(", "));
            }
        }

        w!("}}");
        out
    }
}

fn implode_ranges(bytes: BTreeSet<u8>) -> Vec<String> {
    let mut res = Vec::new();
    let mut i = bytes.iter();
    if let Some(&start) = i.next() {
        let mut start = start;
        let mut prev = start;
        for &cur in i {
            if cur-1 != prev {
                res.push(format!("{:?}-{:?}", start as char, prev as char));
                start = cur;
            }
            prev = cur;
        }
        res
    } else {
        unreachable!();
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct DotOptions {
    pub bold_dict_edges: bool,
    pub suppress_stuck_state: bool,
}

/// Flips a map that represents a non-injective multivalued function
///  to a map that represents the inverse non-injective multivalued function
fn flip_multimap<K: Ord + Clone, V: Ord>(multimap: BTreeMap<K, BTreeSet<V>>)
                                         -> BTreeMap<V, BTreeSet<K>> {
    let mut res = BTreeMap::new();
    for (k, vs) in multimap {
        for v in vs {
            res.entry(v).or_insert_with(BTreeSet::new).insert(k.clone());
        }
    }
    res
}

impl Automaton<Input> for NFA {
    type State = BTreeSet<StateNumber>;

    fn start_state(&self) -> Self::State {
        [AUTO_START].iter().cloned().collect()
    }

    fn stuck_state(&self) -> Self::State {
        [AUTO_STUCK].iter().cloned().collect()
    }

    #[inline]
    fn next_state(&self, states: &Self::State, input: &Input) -> Self::State {
        let mut nxt_states = BTreeSet::new();
        for &state in states {
            if let Some(states) = self.states[state].transitions.get(input) {
                for &nxt_state in states {
                    nxt_states.insert(nxt_state);
                }
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

impl fmt::Debug for NFA {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! w {
            ($($tt:tt)*) => { write!(f, $($tt)*)? }
        }
        for (i, state) in (*self.states).into_iter().enumerate() {
            w!("{}", i);
            if i == AUTO_START {
                w!(" (start)");
            }
            if i == AUTO_STUCK {
                w!(" (stuck)");
            }
            if self.states[i].is_final() {
                w!(" (final)");
            }
            let tr = state.transitions
                .iter()
                .map(|(&c, s)| (c as char, s.clone()))
                .collect::<BTreeMap<char, BTreeSet<StateNumber>>>();
            w!("{:#?},\n", tr);
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

    fn is_final(&self) -> bool {
        !self.pattern_ends.is_empty()
    }

    fn into_dfa(self) -> Result<DFAState, ()> {
        let mut transitions = vec![AUTO_STUCK; 256];
        for (&i, ref sns) in &self.transitions {
            if sns.len() != 1 {
                return Err(());
            }
            match sns.iter().next() {
                Some(&sn) => transitions[i as usize] = sn,
                None => unreachable!(),
            }
        }
        Ok(DFAState::new(transitions.into_boxed_slice(), self.pattern_ends))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static BASIC_DICTIONARY: &'static [&'static str] = &["a", "ab", "bab", "bc", "bca", "c", "caa"];

    #[test]
    fn basic() {
        let nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
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
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY);
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
        let nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_prefixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_postfixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
    }

    #[test]
    fn basic_powerset_ignore_pre_postfixes() {
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
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
        let mut nfa = NFA::from_dictionary(BASIC_DICTIONARY).powerset_construction();
        nfa.ignore_postfixes();
        nfa.ignore_prefixes();
        for (patt_no, &word) in BASIC_DICTIONARY.iter().enumerate() {
            assert!(nfa.apply(word.as_bytes()).contains(&patt_no));
        }
        assert!(!nfa.apply("bbc".as_bytes()).is_empty());
        assert!(!nfa.apply("abb".as_bytes()).is_empty());
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
        let dnfa: &NFA = &nfa.powerset_construction();

        assert!(Automaton::find(dnfa, haystack.as_bytes()).next().is_none());
    }

    static HAYSTACK_SHERLOCK: &'static str = include_str!("../benches/sherlock.txt");

    #[test]
    fn nfa_from_bench_sherlock_alt1() {
        let needles = vec!["Sherlock", "Street"];
        let count = 158;

        let haystack = HAYSTACK_SHERLOCK;

        let mut nfa = NFA::from_dictionary(needles);
        nfa.ignore_prefixes();

        assert_eq!(count, nfa.find(haystack.as_bytes()).count());
    }

    #[test]
    fn dnfa_from_bench_sherlock_alt1() {
        let needles = vec!["Sherlock", "Street"];
        let count = 158;

        let haystack = HAYSTACK_SHERLOCK;

        let mut nfa = NFA::from_dictionary(needles);
        nfa.ignore_prefixes();
        let dnfa = nfa.powerset_construction();

        assert_eq!(count, dnfa.find(haystack.as_bytes()).count());
    }
}
