use std::fmt::Debug;

pub const AUTO_START: usize = 0;

pub trait Automaton<Input, Payload> {
    type State: Debug;

    fn start_state() -> Self::State;

    fn next_state(&self, state: &Self::State, input: &Input) -> Self::State;

    fn get_match(&self, state: &Self::State, text_offset: usize) -> Option<Match<Payload>>;

    fn find<'i, 'a>(&'a self, s: &'i [Input]) -> Matches<'i, 'a, Input, Payload, Self>
        where Self: Sized
    {
        Matches {
            aut: self,
            input: s,
            offset: 0,
            state: Self::start_state(),
        }
    }
}

/// Records a match in the search text.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Match<Payload> {
    /// The payload of the automaton
    pub payload: Payload,
    /// The ending byte offset of the match in the search text.
    pub end: usize,
}

/// An iterator of non-overlapping matches for in-memory text.
///
/// This iterator yields `Match` values.
#[derive(Debug)]
pub struct Matches<'i, 'a, Input: 'i, Payload, A: 'a + Automaton<Input, Payload>> {
    aut: &'a A,
    input: &'i [Input],
    offset: usize,
    state: A::State,
}

impl<'i, 'a, Input, Payload, A: Automaton<Input, Payload>> Iterator
    for Matches<'i, 'a, Input, Payload, A> {
    type Item = Match<Payload>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut offset = self.offset;
        while offset < self.input.len() {
            self.state = self.aut.next_state(&self.state, &self.input[offset]);
            offset += 1;
            if let Some(m) = self.aut.get_match(&self.state, 0) {
                self.offset = offset;
                return Some(m);
            }
        }
        None
    }
}
