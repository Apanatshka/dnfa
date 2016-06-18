# Implementation ideas

- Resolve indirections for states [DDFA]
- Final state info in state (probably better for multiple match search) [DDFA, forced to basically since there's no inverse of Ptr::offset]
- Final state info outside of state (rarely used in single match search) [NFA/DFA]
- Optimise match of literals (completely predictable next bytes)
- Merge ranges of inputs to the same state [NFA: BTreeMap]
- Compact final state [BitVec, but it doesn't compress]
- Experiment with different fast and compact lookups for final states and for input alphabet (https://en.wikipedia.org/wiki/Sparse_array, https://en.wikipedia.org/wiki/Bit_array#Advantages_and_disadvantages, (Reduced Ordered) Binary Decision Diagrams, maybe Zero-Suppressed)
- Write DFA as linear algebra using adjacency matrix (https://github.com/vbarrielle/sprs - sparse linear algebra library for rust), if that's even possible.. If so, can we do matrix multiplication to resolve multiple steps? Probably not, too farfetched. 