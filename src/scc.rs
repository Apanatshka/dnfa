use std::collections::HashMap;

pub struct SccMutState {
    count: usize, // Index
    index: Vec<usize>, // StateRef -> Index
    lowlink: Vec<usize>, // StateRef -> Index
    scc_stack: Vec<usize>, // Stack<StateRef>
    scc_set: HashMap<usize, usize>, // Map<StateRef, StackOffset> of the above stack
    sccs: Vec<Vec<usize>>, // Stack<Set<StateRef>> the SCCs in reverse topo order
}

impl SccMutState {
    pub fn new(states: usize) -> Self {
        SccMutState {
            count: 0,
            index: vec![::std::usize::MAX; states],
            lowlink: vec![::std::usize::MAX; states],
            scc_stack: Vec::new(),
            scc_set: HashMap::new(),
            sccs: Vec::new(),
        }
    }

    #[inline]
    pub fn visited(&self, st_ref: usize) -> bool {
        self.index[st_ref] != ::std::usize::MAX
    }

    #[inline]
    pub fn scc_seed(&self, st_ref: usize) -> bool {
        self.index[st_ref] == self.lowlink[st_ref]
    }

    pub fn init(&mut self, st_ref: usize) {
        self.index[st_ref] = self.count;
        self.lowlink[st_ref] = self.count;
        self.count += 1;
        self.scc_set.insert(st_ref, self.scc_stack.len());
        self.scc_stack.push(st_ref);
    }

    pub fn update(&mut self, from: usize, to: usize) {
        self.lowlink[from] = ::std::cmp::min(self.lowlink[from], self.lowlink[to]);
    }

    pub fn next_state<I: Iterator<Item = usize>>(&mut self,
                                                 from: usize,
                                                 iter: &mut I)
                                                 -> Option<usize> {
        // Note that the `iter` doesn't have to be fully traversed
        for to in iter {
            if !self.visited(to) {
                return Some(to);
            } else if self.scc_set.contains_key(&to) {
                self.update(from, to);
            }
        }
        None
    }

    pub fn construct_scc(&mut self, from: usize) {
        if self.scc_seed(from) {
            let offset = self.scc_stack[from];
            let scc = self.scc_stack.split_off(offset);
            for &st_ref in &scc {
                self.scc_stack.remove(st_ref);
            }
            self.sccs.push(scc);
        }
    }

    pub fn sccs(self) -> Vec<Vec<usize>> {
        self.sccs
    }

    pub fn sccs_mapping(self) -> Vec<usize> {
        self.lowlink
    }

    pub fn sccs_and_mapping(self) -> (Vec<Vec<usize>>, Vec<usize>) {
        (self.sccs, self.lowlink)
    }
}
