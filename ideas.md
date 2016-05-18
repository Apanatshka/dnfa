# Implementation ideas

- Resolve indirections for states
- Final state info in state (simple)
- Final state info outside of state (rarely used in full search)
- Compact final state
- Experiment with different fast and compact lookups for final states and for input alphabet (https://en.wikipedia.org/wiki/Sparse_array, https://en.wikipedia.org/wiki/Bit_array#Advantages_and_disadvantages)
- Write DFA as linear algebra using adjacency matrix (https://github.com/vbarrielle/sprs - sparse linear algebra library for rust)