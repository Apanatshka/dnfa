extern crate dnfa;

use dnfa::nfa::{NFA};

fn main() {
    let dictionary = &["a", "ab", "bab", "bc", "bca", "c", "caa"];
    let mut nfa = NFA::from_dictionary(dictionary);
    println!("nfa");
    // println!("{}", nfa);
    for &word in dictionary {
        println!("{} -> {}", word, nfa.apply(word.as_bytes()));
    }
    println!("aab -> {}", nfa.apply("aab".as_bytes()));
    nfa.ignore_prefixes();
    println!("ignore-prefixes");
    println!("aab -> {}", nfa.apply("aab".as_bytes()));
    let dnfa = nfa.powerset_construction();
    println!("dnfa");
    // println!("{}", dnfa);
    for &word in dictionary {
        println!("{} -> {}", word, dnfa.apply(word.as_bytes()));
    }
    println!("aab -> {}", dnfa.apply("aab".as_bytes()));
    println!("abb -> {}", dnfa.apply("abb".as_bytes()));
    let mut nfa = NFA::from_dictionary(dictionary);
    nfa.ignore_prefixes();
    nfa.ignore_postfixes();
    let dfa = nfa.powerset_construction().freeze().unwrap();
    println!("dfa");
    // println!("{}", dfa);
    for &word in dictionary {
        println!("{} -> {}", word, dfa.apply(word.as_bytes()));
    }
    println!("ignore-postfixes");
    println!("abb -> {}", dfa.apply("abb".as_bytes()));
    let ddfa = dfa.into_ddfa().unwrap();
    println!("ddfa");
    // println!("{}", ddfa);
    for &word in dictionary {
        println!("{} -> {}", word, ddfa.apply(word.as_bytes()));
    }
    println!("abb -> {}", ddfa.apply("abb".as_bytes()));
}
