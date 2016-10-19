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
    fn new(transitions: HashMap<Input, HashSet<StateRef>>, payload: Option<Payload>) -> Self {
        NFAHashState {
            transitions: transitions,
            payload: payload,
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

// NFAE helper types
type Epsilons = Vec<HashSet<usize>>;
type EpsilonsRef<'a> = &'a [HashSet<usize>];
type EpsilonsMutRef<'a>  = &'a mut [HashSet<usize>];
type NFAStates<Input, Payload> = Vec<NFAHashState<Input, usize, Payload>>;
type NFAStatesRef<'a, Input, Payload> = &'a [NFAHashState<Input, usize, Payload>];
type NFAStatesMutRef<'a, Input, Payload> = &'a mut [NFAHashState<Input, usize, Payload>];
type RevEpsilons = Vec<(usize, HashSet<usize>)>;
type Cycle = HashSet<usize>;

impl<Input: Eq + Hash + Clone, Payload: Clone> NFAE<Input, Payload> {
    fn split_transitions(&self) -> (Epsilons, NFAStates<Input, Payload>) {
        let epsilons: Epsilons = self.states
            .iter()
            .map(|st| st.transitions.get(&None).cloned().unwrap_or_else(HashSet::new))
            .collect();

        let states: NFAStates<Input, Payload> = self.states
            .iter()
            .map(|st| {
                let transitions = st.transitions
                    .iter()
                    .filter_map(|(inp, st_ref)| inp.clone().map(|k| (k.clone(), st_ref.clone())))
                    .collect();
                NFAHashState::new(transitions, st.payload.clone())
            })
            .collect();

        (epsilons, states)
    }

    fn build_reverse_epsilons(epsilons: EpsilonsRef) -> RevEpsilons {
        let mut rev_epsilons: RevEpsilons =
            iter::repeat(HashSet::new()).enumerate().take(epsilons.len()).collect();
        for (n, eps) in epsilons.iter().enumerate() {
            for &e in eps {
                rev_epsilons[e].1.insert(n);
            }
        }
        rev_epsilons.retain(|t| !t.1.is_empty());
        rev_epsilons
    }

    fn remove_outer_epsilons(mut rev_epsilons: RevEpsilons,
                             epsilons: EpsilonsMutRef,
                             states: NFAStatesMutRef<Input, Payload>,
                             exclude: &Cycle)
                             -> RevEpsilons {
        rev_epsilons.sort_by_key(|v| epsilons[v.0].len());
        rev_epsilons.into_iter()
            .filter(|&(n, ref rev_eps)| {
                if epsilons[n].is_empty() {
                    for &e in rev_eps.difference(exclude) {
                        for tr in states[n].transitions.clone() {
                            states[e]
                                .transitions
                                .entry(tr.0)
                                .or_insert_with(HashSet::new)
                                .extend(tr.1);
                        }
                        epsilons[e].remove(&n);
                    }
                }
                epsilons[n].is_empty()
            })
            .collect()
    }

    /// Finds a cycle by walking from the given start_state to all other states it can reach
    // TODO: Make sure no line into the cycle is included
    fn find_cycle(start_state: usize, epsilons: EpsilonsMutRef) -> Cycle {
        let mut cycle = HashSet::new();
        let mut todo = vec![start_state];
        while let Some(state) = todo.pop() {
            if !cycle.insert(state) {
                todo.extend(mem::replace(&mut epsilons[state], HashSet::new()));
            }
        }
        cycle
    }

    fn combine_cycle_transitions(cycle: &Cycle,
                                 states: NFAStatesRef<Input, Payload>)
                                 -> HashMap<Input, HashSet<usize>> {
        let mut trans_iter = cycle.iter().map(|&n| &states[n].transitions);
        let mut transitions = trans_iter.next().unwrap_or_else(|| unreachable!()).clone();
        for trans in trans_iter {
            for tr in trans.clone() {
                transitions.entry(tr.0).or_insert_with(HashSet::new).extend(tr.1);
            }
        }
        transitions
    }

    pub fn naive_epsilon_closure(&self) -> NFA<Input, Payload> {
        let (epsilons, mut states) = self.split_transitions();

        for (n, eps) in epsilons.iter().enumerate() {
            for &e in eps {
                for (inp, st_ref) in states[e].transitions.clone() {
                    states[n].transitions.entry(inp).or_insert_with(HashSet::new).extend(st_ref);
                }
            }
        }

        NFA {
            alphabet: self.alphabet.clone(),
            states: states,
        }
    }

    /// Removes epsilon transitions by adding direct transitions
    /// Keeps the amount of states equal, cycles just all get the same transitions
    // Implementation:
    // 1. Get all states with normal transitions, the epsilon transitions, and their reverse.
    // 2. Remove epsilons in a reverse topological order.
    // 3. Detect cycles when no (reverse) epsilons are removed.
    // 4. Get cycle and remove its epsilons.
    // 5. Set cycle states to combined transitions.
    // 6. Remove more epsilons in reverse topo order while using the cycle to avoid extra work.
    //    (Because the rev_epsilons still have the cycle in there)
    pub fn epsilon_closure(&self) -> NFA<Input, Payload> {
        let (mut epsilons, mut states) = self.split_transitions();
        let mut rev_epsilons = Self::build_reverse_epsilons(&epsilons);

        while !rev_epsilons.is_empty() {
            let initial_len = rev_epsilons.len();

            let no_cycle = HashSet::new();
            rev_epsilons =
                Self::remove_outer_epsilons(rev_epsilons, &mut epsilons, &mut states, &no_cycle);

            if initial_len == rev_epsilons.len() {
                let cycle = Self::find_cycle(rev_epsilons[0].0, &mut epsilons);

                let transitions = Self::combine_cycle_transitions(&cycle, &states);

                for &n in &cycle {
                    states[n].transitions = transitions.clone();
                }

                rev_epsilons =
                    Self::remove_outer_epsilons(rev_epsilons, &mut epsilons, &mut states, &cycle);
            }
        }

        NFA {
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
