use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::iter;

use automaton::{AUTO_START, Automaton, Match};

#[derive(Clone)]
struct NFAHashState<Input: Eq + Hash, StateNumber, Payload> {
    transitions: HashMap<Input, HashSet<StateNumber>>,
    payload: Option<Payload>,
}

pub struct NFA<Input: Eq + Hash, Payload> {
    alphabet: Vec<Input>,
    states: Vec<NFAHashState<Input, usize, Payload>>,
}

impl NFAHashState<u8, usize, ()> {
    fn new() -> Self {
        NFAHashState {
            transitions: HashMap::new(),
            payload: None,
        }
    }
}

impl NFA<u8, ()> {
    pub fn new() -> Self {
        NFA {
            alphabet: Vec::new(),
            states: Vec::new(),
        }
    }
}

impl<Input: Eq + Hash, Payload: Clone> Automaton<Input, Payload> for NFA<Input, Payload> {
    type State = HashSet<usize>;

    #[inline]
    fn start_state() -> Self::State {
        iter::once(AUTO_START).collect()
    }

    #[inline]
    fn next_state(&self, states: &Self::State, input: &Input) -> Self::State {
        let mut nxt_states = HashSet::new();
        for &state in states {
            self.states[state].transitions.get(input).map(|states| nxt_states.extend(states));
        }
        nxt_states
    }

    #[inline]
    fn get_match(&self, states: &Self::State, text_offset: usize) -> Option<Match<Payload>> {
        for &state in states {
            if let Some(ref payload) = self.states[state].payload {
                return Some(Match {
                    payload: payload.clone(),
                    end: text_offset,
                });
            }
        }
        None
    }
}
