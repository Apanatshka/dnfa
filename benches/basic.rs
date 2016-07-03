#![feature(test)]

extern crate test;
extern crate dnfa;

use std::iter;

static HAYSTACK_RANDOM: &'static str = include_str!("random.txt");

fn haystack_same(letter: char) -> String {
    iter::repeat(letter).take(10000).collect()
}

// A naive multi-pattern search.
// We use this to benchmark *throughput*, so it should never match anything.
fn naive_find(needles: &[String], haystack: &str) -> bool {
    for hi in 0..haystack.len() {
        let rest = &haystack.as_bytes()[hi..];
        for needle in needles {
            let needle = needle.as_bytes();
            if needle.len() > rest.len() {
                continue;
            }
            if needle == &rest[..needle.len()] {
                // should never happen in throughput benchmarks.
                return true;
            }
        }
    }
    false
}

macro_rules! basic_benches {
    ($prefix:ident, $bench_no_match:expr) => {
        mod $prefix {
            #![allow(unused_imports)]
            use super::{HAYSTACK_RANDOM, haystack_same, naive_find};
            use dnfa::nfa::{NFA};
            use dnfa::dfa::{DFA, DDFA};
            use dnfa::automaton::{Automaton};

            use test::Bencher;

            #[bench]
            fn one_byte(b: &mut Bencher) {
                $bench_no_match(b, vec!["a"], &haystack_same('z'));
            }

            #[bench]
            fn one_prefix_byte_no_match(b: &mut Bencher) {
                $bench_no_match(b, vec!["zbc"], &haystack_same('y'));
            }

            #[bench]
            fn one_prefix_byte_every_match(b: &mut Bencher) {
                // We lose the benefit of `memchr` because the first byte matches
                // in every position in the haystack.
                $bench_no_match(b, vec!["zbc"], &haystack_same('z'));
            }

            #[bench]
            fn one_prefix_byte_random(b: &mut Bencher) {
                $bench_no_match(b, vec!["zbc\x00"], HAYSTACK_RANDOM);
            }

            #[bench]
            fn two_bytes(b: &mut Bencher) {
                $bench_no_match(b, vec!["a", "b"], &haystack_same('z'));
            }

            #[bench]
            fn two_diff_prefix(b: &mut Bencher) {
                $bench_no_match(b, vec!["abcdef", "bmnopq"], &haystack_same('z'));
            }

            #[bench]
            fn two_one_prefix_byte_every_match(b: &mut Bencher) {
                $bench_no_match(b, vec!["zbcdef", "zmnopq"], &haystack_same('z'));
            }

            #[bench]
            fn two_one_prefix_byte_no_match(b: &mut Bencher) {
                $bench_no_match(b, vec!["zbcdef", "zmnopq"], &haystack_same('y'));
            }

            #[bench]
            fn two_one_prefix_byte_random(b: &mut Bencher) {
                $bench_no_match(b, vec!["zbcdef\x00", "zmnopq\x00"], HAYSTACK_RANDOM);
            }

            #[bench]
            fn ten_bytes(b: &mut Bencher) {
                let needles = vec!["a", "b", "c", "d", "e",
                                   "f", "g", "h", "i", "j"];
                $bench_no_match(b, needles, &haystack_same('z'));
            }

            #[bench]
            fn ten_diff_prefix(b: &mut Bencher) {
                let needles = vec!["abcdef", "bbcdef", "cbcdef", "dbcdef",
                                   "ebcdef", "fbcdef", "gbcdef", "hbcdef",
                                   "ibcdef", "jbcdef"];
                $bench_no_match(b, needles, &haystack_same('z'));
            }

            #[bench]
            fn ten_one_prefix_byte_every_match(b: &mut Bencher) {
                let needles = vec!["zacdef", "zbcdef", "zccdef", "zdcdef",
                                   "zecdef", "zfcdef", "zgcdef", "zhcdef",
                                   "zicdef", "zjcdef"];
                $bench_no_match(b, needles, &haystack_same('z'));
            }

            #[bench]
            fn ten_one_prefix_byte_no_match(b: &mut Bencher) {
                let needles = vec!["zacdef", "zbcdef", "zccdef", "zdcdef",
                                   "zecdef", "zfcdef", "zgcdef", "zhcdef",
                                   "zicdef", "zjcdef"];
                $bench_no_match(b, needles, &haystack_same('y'));
            }

            #[bench]
            fn ten_one_prefix_byte_random(b: &mut Bencher) {
                let needles = vec!["zacdef\x00", "zbcdef\x00", "zccdef\x00",
                                   "zdcdef\x00", "zecdef\x00", "zfcdef\x00",
                                   "zgcdef\x00", "zhcdef\x00", "zicdef\x00",
                                   "zjcdef\x00"];
                $bench_no_match(b, needles, HAYSTACK_RANDOM);
            }
        }
    }
}

basic_benches!(naive, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let needles: Vec<String> = needles.into_iter().map(String::from).collect();
    b.iter(|| assert!(!naive_find(&needles, haystack)));
});

basic_benches!(nfa_direct, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();

    b.iter(|| assert!(nfa.find(haystack.as_bytes()).next().is_none()));
});

basic_benches!(nfa_boxed, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa: NFA = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let nfa: &NFA = &nfa;

    b.iter(|| assert!(Automaton::find(nfa, haystack.as_bytes()).next().is_none()));
});

basic_benches!(dnfa_direct, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dnfa = nfa.powerset_construction();

    b.iter(|| assert!(dnfa.find(haystack.as_bytes()).next().is_none()));
});

basic_benches!(dnfa_boxed, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dnfa: &NFA = &nfa.powerset_construction();

    b.iter(|| assert!(Automaton::find(dnfa, haystack.as_bytes()).next().is_none()));
});

basic_benches!(dfa_direct, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dfa = nfa.powerset_construction().into_dfa().unwrap();

    b.iter(|| assert!(dfa.find(haystack.as_bytes()).next().is_none()));
});

basic_benches!(dfa_boxed, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dfa: &DFA = &nfa.powerset_construction().into_dfa().unwrap();

    b.iter(|| assert!(Automaton::find(dfa, haystack.as_bytes()).next().is_none()));
});

basic_benches!(ddfa_direct, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let ddfa = nfa.powerset_construction().into_dfa().unwrap().into_ddfa().unwrap();

    b.iter(|| assert!(ddfa.find(haystack.as_bytes()).next().is_none()));
});

basic_benches!(ddfa_boxed, |b: &mut Bencher, needles: Vec<&str>, haystack: &str| {
    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let ddfa: &DDFA = &nfa.powerset_construction().into_dfa().unwrap().into_ddfa().unwrap();

    b.iter(|| assert!(Automaton::find(ddfa, haystack.as_bytes()).next().is_none()));
});