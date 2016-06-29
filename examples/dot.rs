extern crate dnfa;

use std::env;

use dnfa::nfa::*;
//use dnfa::dfa::*;

fn main() {
    let dict: Vec<String> = env::args().skip(1).collect();
    let mut nfa = NFA::from_dictionary(dict);
    nfa.ignore_prefixes();
    let nfa = nfa.powerset_construction();
    println!("{}", nfa.dot().trim());
}