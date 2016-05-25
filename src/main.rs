mod lib;

use lib::nfa::{NFA, StateNumberSet, powerset_construction, NFA_START};
use lib::dfa::{DFA_STUCK, DFA_START};

fn main() {
    dict_nfa();
    dict_dnfa();
    dict_dfa();
}

fn dict_nfa() {
    fn filter_fn((c, x): (usize, StateNumberSet)) -> Option<(usize, StateNumberSet)> {
        if x.len() == 1 && x[0] == NFA_START || x.len() == 0 {
            None
        } else {
            Some((c, x))
        }
    }

    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let nfa = NFA::from_dictionary(dictionary);
    println!("{}", nfa)
}

fn dict_dnfa() {
    fn filter_fn((c, x): (usize, StateNumberSet)) -> Option<(usize, StateNumberSet)> {
        if x.len() == 1 && x[0] == DFA_START || x.len() == 0 {
            None
        } else {
            Some((c, x))
        }
    }

    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let nfa = powerset_construction(NFA::from_dictionary(dictionary));
    println!("{}", nfa)
}

fn dict_dfa() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let dfa = powerset_construction(NFA::from_dictionary(dictionary)).freeze().unwrap();
    println!("{}", dfa)
}
