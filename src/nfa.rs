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
    /// Removes epsilon transitions by adding direct transitions
    /// Keeps the amount of states equal, cycles just all get the same transitions
    /* Implementation:
    1. Clones all states while keeping only the normal transitions.
    2. Gets all epsilon transitions (and their reverse) while cloning states.
    3. Uses reverse epsilons to remove epsilons in a topological order.
    4. Detects cycles by having reverse epsilons left that cannot be removed.
    5. Uses the forward epsilons to find a cycle and remove it.
    6. The cycle cannot be easily removed from the reverse epsilons list, so we duplicate some code.
    */
    pub fn epsilon_closure(&self) -> NFA<Input, Payload> {
        macro_rules! add_transitions {
            ($one:expr, $other:expr) => {
                for tr in $other {
                    $one.entry(tr.0).or_insert_with(HashSet::new).extend(tr.1);
                }
            }
        }

        let mut epsilons: Vec<HashSet<usize>> =
            iter::repeat(HashSet::new()).take(self.states.len()).collect();
        let mut rev_epsilons: Vec<(usize, HashSet<usize>)> =
            iter::repeat(HashSet::new()).enumerate().take(self.states.len()).collect();

        let mut states: Vec<NFAHashState<Input, usize, Payload>> = self.states
            .iter()
            .enumerate()
            .map(|(n, st)| {
                NFAHashState {
                    transitions: {
                        let mut transitions: HashMap<Input, HashSet<usize>> = HashMap::new();
                        for (k, v) in &st.transitions {
                            if let Some(k) = k.clone() {
                                transitions.insert(k, v.clone());
                            } else {
                                epsilons[n].extend(v);
                                for st_ref in v {
                                    rev_epsilons[*st_ref].1.insert(n);
                                }
                            }
                        }
                        transitions
                    },
                    payload: st.payload.clone(),
                }
            })
            .collect();

        rev_epsilons.retain(|t| !t.1.is_empty());

        let mut new_rev_epsilons = Vec::new();

        while !rev_epsilons.is_empty() {
            rev_epsilons.sort_by_key(|v| epsilons[v.0].len());

            let mut change = false;

            for &mut (n, ref mut rev_eps) in rev_epsilons.iter_mut() {
                if epsilons[n].is_empty() {
                    change = true;
                    for &e in rev_eps.iter() {
                        let n_trans = states[n].transitions.clone();
                        add_transitions!(states[e].transitions, n_trans);
                        epsilons[e].remove(&n);
                    }
                } else {
                    new_rev_epsilons.push((n, mem::replace(rev_eps, HashSet::new())));
                }
            }

            rev_epsilons.clear();
            mem::swap(&mut rev_epsilons, &mut new_rev_epsilons);

            // if epsilon cycle
            if !change && !rev_epsilons.is_empty() {
                // find cycle
                let cycle_node = rev_epsilons[0].0;
                let mut cycle = HashSet::new();
                let mut todo = vec![cycle_node];
                while let Some(node) = todo.pop() {
                    // if new cycle node found
                    if !cycle.insert(node) {
                        todo.extend(mem::replace(&mut epsilons[node], HashSet::new()));
                    }
                }

                // get transitions of cycle
                let transitions = {
                    let mut trans_iter = cycle.iter().map(|&n| &states[n].transitions);
                    let mut transitions =
                        trans_iter.next().unwrap_or_else(|| unreachable!()).clone();
                    for trans in trans_iter {
                        add_transitions!(transitions, trans.clone());
                    }
                    transitions
                };

                // set cycle transitions
                for &n in &cycle {
                    states[n].transitions = transitions.clone();
                }

                for &mut (n, ref mut rev_eps) in rev_epsilons.iter_mut() {
                    if epsilons[n].is_empty() {
                        change = true;
                        for &e in rev_eps.difference(&cycle) {
                            let n_trans = states[n].transitions.clone();
                            add_transitions!(states[e].transitions, n_trans);
                            epsilons[e].remove(&n);
                        }
                    } else {
                        new_rev_epsilons.push((n, mem::replace(rev_eps, HashSet::new())));
                    }
                }

                rev_epsilons.clear();
                mem::swap(&mut rev_epsilons, &mut new_rev_epsilons);
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
