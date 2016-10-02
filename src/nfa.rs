use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::BTreeSet;
use std::hash::Hash;
use std::iter;
use std::mem;

use automaton::{AUTO_START, Automaton, Match};

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

    pub fn powerset_construction<F>(&self, payload_fold: &F) -> Self
        where F: Fn(Option<Payload>, &Option<Payload>) -> Option<Payload>
    {
        type StateRef = usize;

        let mut states = vec![NFAHashState::new()];
        let mut states_map: HashMap<BTreeSet<StateRef>, StateRef> = HashMap::new();
        let cur_states: BTreeSet<StateRef> = iter::once(AUTO_START).collect();

        states[AUTO_START].payload = self.states[AUTO_START].payload.clone();
        states_map.insert(cur_states.clone(), AUTO_START);

        psc_rec_helper(self,
                       &mut states,
                       &mut states_map,
                       cur_states,
                       AUTO_START,
                       payload_fold);

        NFA {
            alphabet: self.alphabet.clone(),
            states: states,
        }
    }
}

fn psc_rec_helper<Input, Payload, F>(nfa: &NFA<Input, Payload>,
                                     states: &mut Vec<NFAHashState<Input, usize, Payload>>,
                                     states_map: &mut HashMap<BTreeSet<usize>, usize>,
                                     cur_states: BTreeSet<usize>,
                                     cur_num: usize,
                                     payload_fold: &F)
    where Input: Eq + Hash + Clone,
          Payload: Clone,
          F: Fn(Option<Payload>, &Option<Payload>) -> Option<Payload>
{
    for symbol in &nfa.alphabet {
        let mut nxt_states = BTreeSet::new();
        nfa._next_state(&cur_states, symbol, &mut nxt_states);

        // Skip the stuck state
        if nxt_states.is_empty() {
            continue;
        }

        let nxt_num = states_map.get(&nxt_states).cloned().unwrap_or_else(|| {
            let nxt_num = states.len();
            let payload = nxt_states.iter()
                .map(|&st| &nfa.states[st].payload)
                .fold(None, payload_fold);
            states.push(NFAHashState::from_payload(payload));
            states_map.insert(nxt_states.clone(), nxt_num);
            psc_rec_helper(nfa, states, states_map, nxt_states, nxt_num, payload_fold);
            nxt_num
        });

        states[cur_num]
            .transitions
            .entry(symbol.clone())
            .or_insert_with(HashSet::new)
            .insert(nxt_num);
    }
}
