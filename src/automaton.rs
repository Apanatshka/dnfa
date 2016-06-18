pub const AUTO_START: usize = 0;
pub const AUTO_STUCK: usize = 1;

pub trait Automaton<Input: Ord> {
    type State;

    fn start_state() -> Self::State;
    fn stuck_state() -> Self::State;

    fn next_state(&self, state: &Self::State, input: &Input) -> Self::State;

    fn has_match(&self, si: &Self::State, outi: usize) -> bool;

    fn get_match(&self, si: &Self::State, outi: usize, texti: usize) -> Match;

    fn find<'i, 'a>(&'a self, s: &'i [Input]) -> Matches<'i, 'a, Input, Self>
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

// This is from burntsushi/aho-corasick.
/// Records a match in the search text.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Match {
    /// The pattern index.
    ///
    /// This corresponds to the ordering in which the matched pattern was
    /// added to the automaton, starting at `0`.
    pub pattern_no: usize,
    /// The starting byte offset of the match in the search text.
    pub start: usize,
    /// The ending byte offset of the match in the search text.
    ///
    /// (This can be re-capitulated with `pati` and adding the pattern's
    /// length to `start`, but it is convenient to have it here.)
    pub end: usize,
}

/// An iterator of non-overlapping matches for in-memory text.
///
/// This iterator yields `Match` values.
#[derive(Debug)]
pub struct Matches<'i, 'a, Input: 'i + Ord, A: 'a + Automaton<Input>> {
    aut: &'a A,
    input: &'i [Input],
    offset: usize,
    state: A::State,
}


impl<'i, 'a, Input: Ord, A: Automaton<Input>> Iterator for Matches<'i, 'a, Input, A> {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        let mut offset = self.offset;
        while offset < self.input.len() {
            self.state = self.aut.next_state(&self.state, &self.input[offset]);
            if self.aut.has_match(&self.state, 0) {
                break;
            }
            offset += 1;
        }
        if offset < self.input.len() {
            self.offset = offset + 1;
            Some(self.aut.get_match(&self.state, 0, offset))
        } else {
            None
        }
    }
}
