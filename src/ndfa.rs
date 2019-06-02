use std::collections::btree_set::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter;

use bit_vec::BitVec;

use crate::dfa::{DFAState, DFA};
use crate::nfa::{Input, StateNumber, START, STUCK};

pub struct NDFA {
    // nfa_StateNumber -> (Input -> Set<nfa_StateNumber>; is_final)
    nfa_states: Vec<(HashMap<Input, HashSet<StateNumber>>, bool)>,
    // dfa_StateNumber -> (Input -> dfa_StateNumber; is_final)
    dfa_states: Vec<(HashMap<Input, StateNumber>, bool)>,
    // nfa_StateNumber -> Set<dfa_StateNumber> where dfa_StateNumber represents nfa_StateNumber (among other nfa_StateNumber)
    corresponding_dfa_states: Vec<HashSet<StateNumber>>,
    // dfa_StateNumber -> Set<nfa_StateNumber>; which set of nfa_StateNumbers the dfa_StateNumber represents
    represents_nfa_states: Vec<HashSet<StateNumber>>,
    // reverse of represents_nfa_states
    nfa_states_to_dfa_state: HashMap<BTreeSet<StateNumber>, StateNumber>,
}

impl NDFA {
    pub fn new() -> Self {
        let mut nfa_to_dfa = HashMap::new();
        nfa_to_dfa.insert([STUCK].iter().cloned().collect(), STUCK);
        nfa_to_dfa.insert([START].iter().cloned().collect(), START);
        NDFA {
            nfa_states: vec![(HashMap::new(), false), (HashMap::new(), false)],
            dfa_states: vec![(HashMap::new(), false), (HashMap::new(), false)],
            corresponding_dfa_states: vec![
                [STUCK].iter().cloned().collect(),
                [START].iter().cloned().collect(),
            ],
            represents_nfa_states: vec![
                [STUCK].iter().cloned().collect(),
                [START].iter().cloned().collect(),
            ],
            nfa_states_to_dfa_state: nfa_to_dfa,
        }
    }

    pub fn start_state() -> StateNumber {
        START
    }

    pub fn stuck_state() -> StateNumber {
        STUCK
    }

    /// You can only add a new state by going from an existing state
    pub fn new_state(&mut self, from: StateNumber, on: Input) -> StateNumber {
        assert!(from < self.nfa_states.len());

        let new_state = self.nfa_states.len();
        self.nfa_states.push((HashMap::new(), false));

        let new_dfa_state = self.dfa_states.len();
        self.dfa_states.push((HashMap::new(), false));

        self.corresponding_dfa_states
            .push([new_dfa_state].iter().cloned().collect());

        let from_state = &mut self.nfa_states[from].0;

        if !(from_state.contains_key(&on)) {
            from_state.insert(on, [new_state].iter().cloned().collect());
            self.represents_nfa_states
                .push([new_state].iter().cloned().collect());
            self.nfa_states_to_dfa_state
                .insert([new_state].iter().cloned().collect(), new_dfa_state);
            for &from_dfa in &self.corresponding_dfa_states[from] {
                let from_dfa_state = &mut self.dfa_states[from_dfa].0;
                assert!(!from_dfa_state.contains_key(&on));
                from_dfa_state.insert(on, new_dfa_state);
            }
        } else {
            let to_states = from_state.get_mut(&on).unwrap();
            to_states.insert(new_state);
            self.represents_nfa_states.push(to_states.clone());
            self.nfa_states_to_dfa_state
                .insert(to_states.iter().cloned().collect(), new_dfa_state);
            for &from_dfa in &self.corresponding_dfa_states[from] {
                let from_dfa_state = &mut self.dfa_states[from_dfa].0;
                if !(from_dfa_state.contains_key(&on)) {
                    from_dfa_state.insert(on, new_dfa_state);
                } else {
                    let &old_dfa_to_state = from_dfa_state.get(&on).unwrap();
                    let nfa_to_states = self.represents_nfa_states[old_dfa_to_state]
                        .iter()
                        .cloned()
                        .chain(iter::once(new_state))
                        .collect();
                    let newer_dfa_state_option = self.nfa_states_to_dfa_state.get(&nfa_to_states);
                    let new_dfa_to_state = if newer_dfa_state_option.is_none() {
                        let newer_dfa_state = self.dfa_states.len();
                        self.dfa_states
                            .push(self.dfa_states[old_dfa_to_state].clone());
                        self.represents_nfa_states
                            .push(nfa_to_states.iter().cloned().collect());
                        self.nfa_states_to_dfa_state
                            .insert(nfa_to_states, newer_dfa_state);
                        newer_dfa_state
                    } else {
                        *newer_dfa_state_option.unwrap()
                    };
                    self.dfa_states[from_dfa].0.insert(on, new_dfa_to_state);
                }
            }
        }

        new_state
    }

    /// You can add more edges between existing states
    pub fn new_edge(&mut self, from: StateNumber, to: StateNumber, on: Input) -> &mut Self {
        assert!(from < self.nfa_states.len());
        assert!(to < self.nfa_states.len());

        let from_state = &mut self.nfa_states[from].0;

        if let Some(to_states) = from_state.get(&on) {
            if to_states.contains(&to) {
                return self;
            }
        }

        for &from_dfa in &self.corresponding_dfa_states[from] {
            let from_dfa_state = &mut self.dfa_states[from_dfa].0;
            let &old_dfa_to_state = from_dfa_state.get(&on).unwrap();
            let mut nfa_to_states =
                from_dfa_state
                    .get(&on)
                    .cloned()
                    .map_or_else(BTreeSet::new, |old_dfa_to_state| {
                        self.represents_nfa_states[old_dfa_to_state]
                            .iter()
                            .cloned()
                            .collect()
                    });
            nfa_to_states.insert(to);
            let newer_dfa_state_option = self.nfa_states_to_dfa_state.get(&nfa_to_states);
            let new_dfa_to_state = if newer_dfa_state_option.is_none() {
                let newer_dfa_state = self.dfa_states.len();
                self.dfa_states
                    .push(self.dfa_states[old_dfa_to_state].clone());
                self.represents_nfa_states
                    .push(nfa_to_states.iter().cloned().collect());
                self.nfa_states_to_dfa_state
                    .insert(nfa_to_states, newer_dfa_state);
                newer_dfa_state
            } else {
                *newer_dfa_state_option.unwrap()
            };
            self.dfa_states[from_dfa].0.insert(on, new_dfa_to_state);
        }

        self
    }

    pub fn mark_final(&mut self, state: StateNumber) -> &mut Self {
        assert!(state < self.nfa_states.len());
        self.nfa_states[state].1 = true;
        for &dfa_state in self.corresponding_dfa_states[state].iter() {
            self.dfa_states[dfa_state].1 = true;
        }
        self
    }

    /// You can finalize the ndfa into a dfa, basically forgetting the nfa part you used to build it
    pub fn finalize(&self) -> DFA {
        let mut finals = BitVec::with_capacity(self.dfa_states.len());

        let states: Vec<_> = self
            .dfa_states
            .iter()
            .map(|state| {
                DFAState::new(
                    {
                        let mut transitions = vec![STUCK; 256];
                        finals.push(state.1);
                        for (&input, &to) in &state.0 {
                            transitions[input as usize] = to;
                        }
                        transitions.into_boxed_slice()
                    },
                    vec![],
                )
            })
            .collect();

        DFA::new(states.into_boxed_slice(), finals, vec![])
    }

    /// Remove unused DFA states created during the build
    pub fn compact(&mut self) -> &mut Self {
        let mut seen_states = HashSet::new();
        seen_states.insert(START);

        let mut worklist = vec![START];
        while let Some(state) = worklist.pop() {
            let from = &self.dfa_states[state];
            for &to in from.0.values() {
                if !seen_states.contains(&to) {
                    seen_states.insert(to);
                    worklist.push(to);
                }
            }
        }

        let mut dropped = 0;
        let mut renumber = HashMap::new();
        for idx in 1..seen_states.len() {
            if !seen_states.contains(&idx) {
                dropped += 1;
            } else if dropped > 0 {
                renumber.insert(idx, idx - dropped);
                self.dfa_states.swap(idx - dropped, idx);
                self.represents_nfa_states.swap(idx - dropped, idx);
                self.nfa_states_to_dfa_state.insert(
                    self.represents_nfa_states[idx - dropped]
                        .iter()
                        .cloned()
                        .collect(),
                    idx - dropped,
                );
            }
        }
        self.dfa_states.truncate(self.dfa_states.len() - dropped);
        for (dfa_state, _) in self.dfa_states.iter_mut() {
            dfa_state.values_mut().for_each(|to_state_ref| {
                *to_state_ref = *renumber.get(to_state_ref).unwrap_or(to_state_ref)
            });
        }
        self.represents_nfa_states
            .truncate(self.represents_nfa_states.len() - dropped);
        for dfa_state in self.corresponding_dfa_states.iter_mut() {
            *dfa_state = dfa_state
                .iter()
                .cloned()
                .map(|dfa_state| *renumber.get(&dfa_state).unwrap_or(&dfa_state))
                .collect();
        }

        self
    }
}
