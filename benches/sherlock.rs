#![feature(test)]

extern crate test;
extern crate dnfa;

// These benchmark tests are derived from the ones in https://github.com/burntsushi/aho-corasick

static HAYSTACK_SHERLOCK: &'static str = include_str!("sherlock.txt");

macro_rules! sherlock_benches {
    ($prefix:ident, $bench_match_count:expr) => {
        mod $prefix {
            #![allow(unused_imports)]
            use super::HAYSTACK_SHERLOCK;
            use dnfa::nfa::{NFA};
            use dnfa::dfa::{DFA, DDFA};
            use dnfa::automaton::{Automaton};

            use test::Bencher;

            #[bench]
            fn alt1(b: &mut Bencher) {
                $bench_match_count(b, 158, vec!["Sherlock", "Street"]);
            }

            #[bench]
            fn alt2(b: &mut Bencher) {
                $bench_match_count(b, 558, vec!["Sherlock", "Holmes"]);
            }

            #[bench]
            fn alt3(b: &mut Bencher) {
                let needles = vec![
                    "Sherlock", "Holmes", "Watson", "Irene", "Adler", "John", "Baker",
                ];
                $bench_match_count(b, 740, needles);
            }

            #[bench]
            fn alt3_nocase(b: &mut Bencher) {
                let needles = vec![
                    "ADL", "ADl", "AdL", "Adl", "BAK", "BAk", "BAK", "BaK", "Bak", "BaK",
                    "HOL", "HOl", "HoL", "Hol", "IRE", "IRe", "IrE", "Ire", "JOH", "JOh",
                    "JoH", "Joh", "SHE", "SHe", "ShE", "She", "WAT", "WAt", "WaT", "Wat",
                    "aDL", "aDl", "adL", "adl", "bAK", "bAk", "bAK", "baK", "bak", "baK",
                    "hOL", "hOl", "hoL", "hol", "iRE", "iRe", "irE", "ire", "jOH", "jOh",
                    "joH", "joh", "sHE", "sHe", "shE", "she", "wAT", "wAt", "waT", "wat",
                    "ſHE", "ſHe", "ſhE", "ſhe",
                ];
                $bench_match_count(b, 1764, needles);
            }
            #[bench]
            fn alt4(b: &mut Bencher) {
                   $bench_match_count(b, 582, vec!["Sher", "Hol"]);
            }

            #[bench]
            fn alt4_nocase(b: &mut Bencher) {
                let needles = vec![
                    "HOL", "HOl", "HoL", "Hol", "SHE", "SHe", "ShE", "She", "hOL", "hOl",
                    "hoL", "hol", "sHE", "sHe", "shE", "she", "ſHE", "ſHe", "ſhE", "ſhe",
                ];
                $bench_match_count(b, 1307, needles);
            }

            #[bench]
            fn alt5(b: &mut Bencher) {
                   $bench_match_count(b, 639, vec!["Sherlock", "Holmes", "Watson"]);
            }

            #[bench]
            fn alt5_nocase(b: &mut Bencher) {
                let needles = vec![
                    "HOL", "HOl", "HoL", "Hol", "SHE", "SHe", "ShE", "She", "WAT", "WAt",
                    "WaT", "Wat", "hOL", "hOl", "hoL", "hol", "sHE", "sHe", "shE", "she",
                    "wAT", "wAt", "waT", "wat", "ſHE", "ſHe", "ſhE", "ſhe",
                ];
                $bench_match_count(b, 1442, needles);
            }
        }
    }
}

sherlock_benches!(nfa_direct, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();

    b.iter(|| assert_eq!(count, nfa.find(haystack.as_bytes()).count()));
});

sherlock_benches!(nfa_boxed, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa: NFA = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let nfa: &NFA = &nfa;

    b.iter(|| assert_eq!(count, Automaton::find(nfa, haystack.as_bytes()).count()));
});

sherlock_benches!(dnfa_direct, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dnfa = nfa.powerset_construction();

    b.iter(|| assert_eq!(count, dnfa.find(haystack.as_bytes()).count()));
});

sherlock_benches!(dnfa_boxed, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dnfa: &NFA = &nfa.powerset_construction();

    b.iter(|| assert_eq!(count, Automaton::find(dnfa, haystack.as_bytes()).count()));
});

sherlock_benches!(dfa_direct, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dfa = nfa.powerset_construction().into_dfa().unwrap();

    b.iter(|| assert_eq!(count, dfa.find(haystack.as_bytes()).count()));
});

sherlock_benches!(dfa_boxed, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let dfa: &DFA = &nfa.powerset_construction().into_dfa().unwrap();

    b.iter(|| assert_eq!(count, Automaton::find(dfa, haystack.as_bytes()).count()));
});

sherlock_benches!(ddfa_direct, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let ddfa = nfa.powerset_construction().into_dfa().unwrap().into_ddfa().unwrap();

    b.iter(|| assert_eq!(count, ddfa.find(haystack.as_bytes()).count()));
});

sherlock_benches!(ddfa_boxed, |b: &mut Bencher, count: usize, needles: Vec<&str>| {
    let haystack = HAYSTACK_SHERLOCK;

    b.bytes = haystack.len() as u64;
    let mut nfa = NFA::from_dictionary(needles);
    nfa.ignore_prefixes();
    let ddfa: &DDFA = &nfa.powerset_construction().into_dfa().unwrap().into_ddfa().unwrap();

    b.iter(|| assert_eq!(count, Automaton::find(ddfa, haystack.as_bytes()).count()));
});
