# Implementation ideas

## Resolve indirections for states

We can skip the pointer arithmetic that we do for every state transition if instead of an offset in the state array we save the pointer to the state in the state array. This is implemented in dfa.rs as the "Direct DFA" or DDFA. 

## Final state info in state

When the check if a state is a final state is done a lot (when we do multiple searches in a text for example), the information whether a state is final should be in the state struct itself since it's accessed a lot. We currently have this for all implementations. 

## Final state info outside of state

When the check if a state is final is relatively rare (when we're doing a single search in a text for example), the information whether a state is final can be packed tightly and stored in the general information of the automaton. This makes the state structs more space-efficient and therefore the whole automaton is more cache-efficient. This idea is currently unused and probably not worth the effort. 

## Optimise match of literals

When a regex has a literal word in it, it has a completely predictable next set bytes when it gets to that literal. There may be a way to optimise that case. Not sure yet..

## Merge ranges of inputs to the same state

When we match on unicode but have the automaton work on bytes, we can get byte ranges that go to the same state. We can save such transitions more compactly (if we give up those fast, byte-indexed transition arrays) when we use a map and only save the inclusive ends of the transition. We can save only the lower end of a range and put the lower end of a stuck state range in there explicitly. This is only inefficient if there are a lot of alternations between bytes that have a transition and bytes that don't. 

## Try different representations for the sparse lookup in transitions

Experiment with different fast and compact lookups for final states and for input alphabet (https://en.wikipedia.org/wiki/Sparse_array, https://en.wikipedia.org/wiki/Bit_array#Advantages_and_disadvantages, (Reduced Ordered) Binary Decision Diagrams, maybe Zero-Suppressed)

## [crazy] Write DFA as linear algebra using adjacency matrix

Probably not even possible. https://github.com/vbarrielle/sprs - sparse linear algebra library for rust.  
