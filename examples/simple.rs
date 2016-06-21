extern crate dnfa;

use dnfa::nfa::{NFA};

fn main() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];

    let mut nfa = NFA::from_dictionary(dictionary);
//    println!("nfa");
//    println!("{}", nfa);
    let dnfa = nfa.powerset_construction();
//    println!("dnfa");
//    println!("{}", dnfa);

    let mut nfa = NFA::from_dictionary(dictionary);
    nfa.ignore_prefixes();
    nfa.ignore_postfixes();
    let dfa = nfa.powerset_construction().freeze().unwrap();
//    println!("dfa");
//    println!("{}", dfa);
    let ddfa = dfa.into_ddfa().unwrap();
//    println!("ddfa");
//    println!("{}", ddfa);
}
