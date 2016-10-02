use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::BTreeSet;
use std::hash::Hash;
use std::iter;
use std::mem;

use automaton::{AUTO_START, Automaton, Match};

// NFAs

#[derive(Clone)]
struct NFAHashState<Input, StateRef, Payload> {
    transitions: HashMap<Input, HashSet<StateRef>>,
    payload: Option<Payload>,
}

#[derive(Clone)]
pub struct NFA<Input, Payload> {
    alphabet: Vec<Input>,
    states: Vec<NFAHashState<Input, usize, Payload>>,
}

impl<Input: Eq + Hash, StateRef, Payload> NFAHashState<Input, StateRef, Payload> {
    fn new() -> Self {
        NFAHashState {
            transitions: HashMap::new(),
            payload: None,
        }
    }

    fn from_payload(payload: Option<Payload>) -> Self {
        NFAHashState {
            transitions: HashMap::new(),
            payload: payload,
        }
    }
}

impl<Input: Eq + Hash, Payload> NFA<Input, Payload> {
    pub fn new() -> Self {
        NFA {
            alphabet: Vec::new(),
            states: Vec::new(),
        }
    }
}

impl<Input: Eq + Hash, Payload: Clone> NFA<Input, Payload> {
    #[inline]
    fn _next_state<'i, 'j, Iter, Ext>(&'j self, states: Iter, symbol: &Input, nxt_states: &mut Ext)
        where Iter: IntoIterator<Item = &'i usize>,
              Ext: Extend<&'j usize>
    {
        for &state in states {
            if let Some(states) = self.states[state].transitions.get(symbol) {
                nxt_states.extend(states);
            }
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
    fn next_state(&self, states: &Self::State, symbol: &Input) -> Self::State {
        let mut nxt_states = HashSet::new();
        self._next_state(states, symbol, &mut nxt_states);
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

impl<Input: Eq + Hash + Clone, Payload: Clone> NFA<Input, Payload> {
    pub fn apply<I: AsRef<[Input]>>(&self, input: I) -> Option<Payload> {
        let mut cur_states = HashSet::new();
        let mut nxt_states = HashSet::new();
        cur_states.insert(AUTO_START);
        for symbol in input.as_ref() {
            self._next_state(&cur_states, symbol, &mut nxt_states);
            // clear + swap: reuses memory.
            // Otherwise same effect as `cur_states = nxt_states; nxt_state = HashSet::new();`
            cur_states.clear();
            mem::swap(&mut cur_states, &mut nxt_states);

            // Return early if "in stuck state"
            if cur_states.is_empty() {
                return None;
            }
        }
        cur_states.iter().filter_map(|&state| self.states[state].payload.clone()).next()
    }

    pub fn powerset_construction<F>(&self, payload_fold: &F) -> DFA<Input, Payload>
        where F: Fn(Option<Payload>, &Option<Payload>) -> Option<Payload>
    {
        type StateRef = usize;

        let mut states = vec![DFAHashState::new()];
        let mut states_map: HashMap<BTreeSet<StateRef>, StateRef> = HashMap::new();
        let cur_states: BTreeSet<StateRef> = iter::once(AUTO_START).collect();

        states[AUTO_START].payload = self.states[AUTO_START].payload.clone();
        states_map.insert(cur_states.clone(), AUTO_START);

        let mut worklist = vec![(cur_states, AUTO_START)];
        while let Some((cur_states, cur_num)) = worklist.pop() {
            for symbol in &self.alphabet {
                let mut nxt_states = BTreeSet::new();
                self._next_state(&cur_states, symbol, &mut nxt_states);

                // Skip the stuck state
                if nxt_states.is_empty() {
                    continue;
                }

                let nxt_num = states_map.get(&nxt_states).cloned().unwrap_or_else(|| {
                    let nxt_num = states.len();
                    let payload = nxt_states.iter()
                        .map(|&st| &self.states[st].payload)
                        .fold(None, payload_fold);
                    states.push(DFAHashState::from_payload(payload));
                    states_map.insert(nxt_states.clone(), nxt_num);
                    worklist.push((nxt_states, nxt_num));
                    nxt_num
                });

                states[cur_num].transitions.insert(symbol.clone(), nxt_num);
            }
        }

        DFA {
            alphabet: self.alphabet.clone(),
            states: states,
        }
    }
}

// DFAs

#[derive(Clone)]
struct DFAHashState<Input, StateRef, Payload> {
    transitions: HashMap<Input, StateRef>,
    payload: Option<Payload>,
}

#[derive(Clone)]
pub struct DFA<Input, Payload> {
    alphabet: Vec<Input>,
    states: Vec<DFAHashState<Input, usize, Payload>>,
}

impl<Input: Eq + Hash, StateRef, Payload> DFAHashState<Input, StateRef, Payload> {
    fn new() -> Self {
        DFAHashState {
            transitions: HashMap::new(),
            payload: None,
        }
    }

    fn from_payload(payload: Option<Payload>) -> Self {
        DFAHashState {
            transitions: HashMap::new(),
            payload: payload,
        }
    }
}

impl<Input: Eq + Hash, Payload> DFA<Input, Payload> {
    pub fn new() -> Self {
        DFA {
            alphabet: Vec::new(),
            states: Vec::new(),
        }
    }
}
