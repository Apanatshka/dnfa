mod lib;

use lib::nfa::{NFA, StateNumberSet, powerset_construction, NFA_START};
use lib::dfa::{DFA_STUCK, DFA_START};

fn main() {
    dict_nfa();
    dict_dnfa();
    dict_dfa();
}

fn f((c, x): (usize, StateNumberSet)) -> Option<(usize, StateNumberSet)> {
    if x.len() == 0 {
        None
    } else {
        Some((c, x))
    }
}

fn dict_nfa() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let nfa = NFA::from_dictionary(dictionary);
    for (i, state) in nfa.states.into_iter().enumerate() {
        print!("{} -> [", i);
        if !state.transitions.is_empty() {
            println!("");
        }
        for (c, tr) in state.transitions.into_iter().enumerate().filter_map(|(c,x)|
            if i == NFA_START && x.len() == 1 && x[0] == NFA_START {
                None
            }  else {
                f((c,x))
            }
        ) {
            println!("  {} -> {:?},", (c as u8) as char, tr)
        }
        print!("]");
        if nfa.finals[i] {
            print!(" -- final state");
        }
        println!(",");
    }
}

fn dict_dnfa() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let nfa = powerset_construction(NFA::from_dictionary(dictionary));
    for (i, state) in nfa.states.into_iter().enumerate() {
        if i == DFA_STUCK {
            continue;
        }
        print!("{} -> [", i);
        if !state.transitions.is_empty() {
            println!("");
        }
        for (c, tr) in state.transitions.into_iter().enumerate().filter_map(|(c,x)|
            if x.len() == 1 && x[0] == DFA_START {
                None
            }  else {
                f((c,x))
            }
        ) {
            println!("  {} -> {:?},", (c as u8) as char, tr)
        }
        print!("]");
        if nfa.finals[i] {
            print!(" -- final state");
        }
        println!(",");
    }
}

fn dict_dfa() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let dfa = powerset_construction(NFA::from_dictionary(dictionary)).freeze().unwrap();
    for (i, state) in (*dfa.states).into_iter().enumerate() {
        if i == DFA_STUCK {
            continue;
        }
        print!("{} -> [", i);
        if !state.transitions.is_empty() {
            println!("");
        }
        for (c, tr) in (*state.transitions).into_iter().enumerate().filter(|&(i,&x)| !(x == DFA_START || x == DFA_STUCK)) {
            println!("  {} -> {:?},", (c as u8) as char, tr)
        }
        print!("]");
        if dfa.finals[i] {
            print!(" -- final state");
        }
        println!(",");
    }
}