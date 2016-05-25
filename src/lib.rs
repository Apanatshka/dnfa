// Explicit paths can be dropped when we drop the binary
#[path="dfa.rs"]
pub mod dfa;
#[path="nfa.rs"]
pub mod nfa;

#[cfg(test)]
mod tests {}
