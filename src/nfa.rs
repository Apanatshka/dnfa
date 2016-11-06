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

#[derive(Clone)]
struct NFAEHashState<Input, StateRef, Payload> {
    transitions: HashMap<Input, HashSet<StateRef>>,
    e_transition: HashSet<StateRef>,
    payload: Option<Payload>,
}

type NFAE<Input, Payload> = FiniteAutomaton<Input, NFAEHashState<Input, usize, Payload>>;

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

impl<Input: Eq + Hash + Clone, StateRef: Clone, Payload: Clone> NFAEHashState<Input,
                                                                              StateRef,
                                                                              Payload> {
    fn drop_epsilons(&self) -> NFAHashState<Input, StateRef, Payload> {
        NFAHashState {
            transitions: self.transitions.clone(),
            payload: self.payload.clone(),
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
    /// States that aren't reachable from `AUTO_START` are preserved (not by design)
    pub fn to_nfa(&self) -> NFA<Input, Payload> {
        // the new states
        let mut states: Vec<NFAHashState<Input, usize, Payload>> = Vec::new();
        // Maps visited stateref to offset in stack
        let mut visiting: HashMap<usize, usize> = HashMap::new();
        // Maps staterefs that are finished to staterefs in the new NFA
        let mut renumbering: Vec<usize> = vec![::std::usize::MAX; self.states.len()];
        // Stack of states we've seen/ are working on.
        let mut stack: Vec<usize> = vec![AUTO_START];
        // Lowest non-visited
        let mut unvisited = 0;

        loop {
            let nfae_st_ref = stack[stack.len() - 1];
            let nfae_st = &self.states[nfae_st_ref];
            let new_state = if !nfae_st.e_transition.is_empty() {
                for &st_ref in &nfae_st.e_transition {
                    match visiting.get(&st_ref) {
                        Some(&::std::usize::MAX) => continue,
                        Some(&offset) => {
                            unimplemented!(); //TODO: loop detected
                            break;
                        }
                        None => {
                            visiting.insert(st_ref, stack.len());
                            stack.push(st_ref);
                            break;
                        }
                    }
                }
                // No more epsilons to do
                Self::eps_state_to_nfa(nfae_st, &renumbering, &states)
            } else {
                if nfae_st_ref == AUTO_START && states.len() != AUTO_START {
                    states[AUTO_START] = nfae_st.drop_epsilons();
                    continue;
                }
                if states.len() == AUTO_START && nfae_st_ref != AUTO_START {
                    states.push(NFAHashState::new());
                }
                nfae_st.drop_epsilons()
            };
            renumbering[nfae_st_ref] = states.len();
            states.push(new_state);
            visiting.remove(&nfae_st_ref);
            stack.pop();
            while renumbering[unvisited] != ::std::usize::MAX && unvisited < self.states.len() {
                unvisited += 1;
            }
            if stack.is_empty() && unvisited < self.states.len() {
                stack.push(unvisited);
            } else {
                break;
            }
        }

        for state in &mut states {
            for (_, st_refs) in &mut state.transitions {
                let new_st_refs = st_refs.iter().map(|&st_ref| renumbering[st_ref]).collect();
                mem::replace(st_refs, new_st_refs);
            }
        }

        NFA {
            alphabet: self.alphabet.clone(),
            states: states,
        }
    }

    fn eps_state_to_nfa(st: &NFAEHashState<Input, usize, Payload>,
                        renumbering: &[usize],
                        states: &[NFAHashState<Input, usize, Payload>])
                        -> NFAHashState<Input, usize, Payload> {
        let mut transitions: HashMap<Input, HashSet<usize>> = st.transitions.clone();
        for &st_ref in &st.e_transition {
            renumbering[st_ref];
            transitions.extend(states[st_ref].transitions.clone());
        }
        NFAHashState {
            transitions: st.transitions.clone(),
            payload: st.payload.clone(),
        }
    }

    /// This is an implementation of Tarjan's Strongly Connected Components algorithm. The nice
    /// property of this SCC algorithm is that it gives the SCC's in reverse topological order.
    /// Note that the normal version of this algorithm is recursive. The implementation below is not
    /// recursive, but instead has an explicit `call_stack`. To make the whole thing a little less
    /// unwieldy, the function is split up into phases that mostly directly transition to each
    /// other. See the `scc_*` function for the different phases. For reference, here is the
    /// recursive version:
    /// ```rust
    /// fn scc_strongconnect(&self,
    ///                      from: usize,
    ///                      index: &mut usize,
    ///                      st_index: &mut [usize],
    ///                      st_lowlink: &mut [usize],
    ///                      scc_stack: &mut Vec<usize>,
    ///                      stack_set: &mut HashSet<usize>,
    ///                      scc_s: &mut Vec<Vec<usize>>) {
    ///     st_index[from] = *index;
    ///     st_lowlink[from] = *index;
    ///     *index += 1;
    ///
    ///     scc_stack.push(from);
    ///     stack_set.insert(from);
    ///
    ///     for &to in &self.states[from].e_transition {
    ///         if st_index[to] == ::std::usize::MAX {
    ///             // `to` will be added to `scc_stack`
    ///             self.scc_strongconnect(to,
    ///                                    index,
    ///                                    st_index,
    ///                                    st_lowlink,
    ///                                    scc_stack,
    ///                                    stack_set,
    ///                                    scc_s);
    ///             // *only* if an SCC if found, `to` is remove from `scc_stack`
    ///             st_lowlink[from] = ::std::cmp::min(st_lowlink[from], st_lowlink[to]);
    ///         } else if stack_set.contains(&to) {
    ///             st_lowlink[from] = ::std::cmp::min(st_lowlink[from], st_index[to]);
    ///         }
    ///     }
    ///
    ///     if st_lowlink[from] == st_index[from] {
    ///         let mut scc = Vec::new();
    ///         while let Some(st_ref) = scc_stack.pop() {
    ///             stack_set.remove(&st_ref);
    ///             scc.push(st_ref);
    ///             if st_ref == from {
    ///                 break;
    ///             }
    ///         }
    ///         scc_s.push(scc);
    ///     }
    /// }
    /// ```
    fn scc(&self) -> Vec<usize> {
        use scc::SccMutState;

        let mut scc_state = SccMutState::new(self.states.len());
        let mut call_stack = Vec::new();

        for st_ref in 0..self.states.len() {
            if !scc_state.visited(st_ref) {
                let mut state = SccState::Init(st_ref);
                loop {
                    if let SccState::Init(from) = state {
                        scc_state.init(from);
                        state = SccState::Dfs(from,
                                              self.states[from]
                                                  .e_transition
                                                  .iter()
                                                  .cloned());
                    }
                    if let SccState::RecCallReturn(from, to, iter) = state {
                        scc_state.update(from, to);
                        state = SccState::Dfs(from, iter);
                    }
                    if let SccState::Dfs(from, mut iter) = state {
                        if let Some(to) = scc_state.next_state(from, &mut iter) {
                            call_stack.push(SccState::RecCallReturn(from, to, iter));
                            state = SccState::Init(to);
                        } else {
                            state = SccState::SccConstruction(from);
                        }
                    }
                    if let SccState::SccConstruction(from) = state {
                        scc_state.construct_scc(from);
                        if let Some(st) = call_stack.pop() {
                            state = st
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        scc_state.sccs_mapping()
    }
}

enum SccState<I: Iterator<Item = usize>> {
    Init(usize),
    Dfs(usize, I),
    RecCallReturn(usize, usize, I),
    SccConstruction(usize),
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
