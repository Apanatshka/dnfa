#![allow(dead_code)]
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
pub struct FiniteAutomaton<Input, State> {
    alphabet: Vec<Input>,
    states: Vec<State>,
}

type NFA<Input, Payload> = FiniteAutomaton<Input, NFAHashState<Input, usize, Payload>>;
type NFAE<Input, Payload> = FiniteAutomaton<Input, NFAHashState<Option<Input>, usize, Payload>>;

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
        where F: Fn(Payload, &Payload) -> Payload
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
                    let payload = {
                        let mut iter = nxt_states.iter()
                            .filter_map(|&st| self.states[st].payload.as_ref());
                        if let Some(first) = iter.next() {
                            Some(iter.fold(first.clone(), payload_fold))
                        } else {
                            None
                        }
                    };
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

impl<Input: Eq + Hash + Clone, Payload: Clone> NFAE<Input, Payload> {
    /// Replaces epsilon transitions with equivalent states/transitions
    /// Cycles are replaced by single states
    pub fn to_nfa(&self) -> NFA<Input, Payload> {
        // the new states
        let mut states: Vec<NFAHashState<Input, usize, Payload>> = Vec::new();
        // Maps visited stateref to offset in stack
        let mut visited: HashMap<usize, usize> = HashMap::new();
        // Maps staterefs that are finished to staterefs in the new NFA
        let mut renumbering: Vec<usize> = vec![::std::usize::MAX; states.len()];
        // Stack of states we've seen/ are working on.
        let mut stack: Vec<usize> = vec![AUTO_START];

        while !stack.is_empty() {
            let nfae_st_ref = stack[stack.len() - 1];
            let nfae_st = &self.states[nfae_st_ref];
            let new_state = if let Some(st_refs) = nfae_st.transitions.get(&None) {
                for &st_ref in st_refs {
                    match visited.get(&st_ref) {
                        Some(&::std::usize::MAX) => continue,
                        Some(&offset) => {
                            unimplemented!(); //TODO: loop detected
                            break;
                        }
                        None => {
                            visited.insert(st_ref, stack.len());
                            stack.push(st_ref);
                            break;
                        }
                    }
                }
                // No more epsilons to do
                Self::eps_state_to_nfa(nfae_st, &renumbering, &states)
            } else {
                if nfae_st_ref == AUTO_START && states.len() != AUTO_START {
                    states[AUTO_START] = Self::state_to_nfa(nfae_st, &renumbering);
                    continue;
                }
                if states.len() == AUTO_START && nfae_st_ref != AUTO_START {
                    states.push(NFAHashState::new());
                }
                Self::state_to_nfa(nfae_st, &renumbering)
            };
            renumbering[nfae_st_ref] = states.len();
            states.push(new_state);
            visited.remove(&nfae_st_ref);
            stack.pop();
            // TODO: Add something new to `stack` when it's empty, so we don't just process the...
            // TODO: ...states reachable from `AUTO_START` with epsilon transitions.
        }

        NFA {
            alphabet: self.alphabet.clone(),
            states: states,
        }
    }

    fn state_to_nfa(st: &NFAHashState<Option<Input>, usize, Payload>,
                    renumbering: &[usize])
                    -> NFAHashState<Input, usize, Payload> {
        NFAHashState {
            transitions: st.transitions
                .iter()
                .filter_map(|(k, v)| {
                    k.as_ref().map(|k| (k.clone(), v.iter().map(|&r| renumbering[r]).collect()))
                })
                .collect(),
            payload: st.payload.clone(),
        }
    }

    fn eps_state_to_nfa(st: &NFAHashState<Option<Input>, usize, Payload>,
                        renumbering: &[usize],
                        states: &[NFAHashState<Input, usize, Payload>])
                        -> NFAHashState<Input, usize, Payload> {
        let mut transitions: HashMap<Input, HashSet<usize>> = HashMap::new();
        for (input, st_refs) in &st.transitions {
            let renumbered = st_refs.iter().map(|&st_ref| renumbering[st_ref]);
            match input.as_ref() {
                Some(input) => {
                    transitions.insert(input.clone(), renumbered.collect());
                }
                None => transitions.extend(
                    renumbered.flat_map(|st_ref| states[st_ref].transitions.clone())),
            }
        }
        NFAHashState {
            transitions: transitions,
            payload: st.payload.clone(),
        }
    }
}

// DFAs

#[derive(Clone)]
struct DFAHashState<Input, StateRef, Payload> {
    transitions: HashMap<Input, StateRef>,
    payload: Option<Payload>,
}

type DFA<Input, Payload> = FiniteAutomaton<Input, DFAHashState<Input, usize, Payload>>;

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
