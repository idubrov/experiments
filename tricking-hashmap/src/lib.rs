#![feature(test)]
extern crate rand;
extern crate test;

use rand::prelude::*;
use std::borrow::Borrow;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use test::Bencher;

pub struct ShortKey<'a>(&'a str);

pub trait Key {
    // Return the tuple of the first byte of a key plus the rest of the key
    fn key(&self) -> (Option<u8>, &[u8]);
}

impl<'a> Key for ShortKey<'a> {
    fn key(&self) -> (Option<u8>, &[u8]) {
        // Pretend we are a String with "_" first character!
        (Some(b'_'), self.0.as_bytes())
    }
}

impl Key for String {
    fn key(&self) -> (Option<u8>, &[u8]) {
        let bytes = self.as_bytes();
        if bytes.is_empty() {
            (None, b"")
        } else {
            // Split the first byte and return a byte slice
            // corresponding to the rest of the string
            (Some(bytes[0]), &bytes[1..])
        }
    }
}

impl<'a> Borrow<Key + 'a> for String {
    fn borrow(&self) -> &(Key + 'a) {
        self
    }
}

impl<'a> Eq for (Key + 'a) {}

impl<'a> PartialEq for (Key + 'a) {
    fn eq(&self, other: &Key) -> bool {
        self.key() == other.key()
    }
}

impl<'a> Hash for (Key + 'a) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.key() {
            (Some(c), rest) => {
                state.write_u8(c);
                state.write(rest);
                state.write_u8(0xff)
            }
            (None, s) => s.hash(state),
        }
    }
}

// Examples

#[test]
fn example1() {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("hello".to_string(), "world".to_string());
    assert_eq!("world", map.get("hello").unwrap());
}

#[test]
fn example2() {
    let mut map: HashMap<(String, String), bool> = HashMap::new();
    map.insert(("hello".to_string(), "world".to_string()), true);
    // Extra memory allocation to create lookup key!
    assert!(
        map.get(&("hello".to_string(), "world".to_string()))
            .unwrap()
    );
}

#[test]
fn example3() {
    // This is what was given to us!
    let key = "hello";
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("hello".to_string(), "world".to_string());
    map.insert("_hello".to_string(), "people".to_string());
    // Allocating a new String to create a lookup key!
    assert_eq!("people", map.get(&format!("_{}", key)).unwrap());
}

#[test]
fn example4() {
    let mut map = HashMap::new();
    map.insert("_hello".to_string(), "world".to_string());
    let lookup_key = &ShortKey("hello") as &Key;
    assert_eq!("world", map.get(lookup_key).unwrap());
}

// Benchmarks

#[bench]
fn with_allocations(bencher: &mut Bencher) {
    let keys = gen_keys(LEN, KEY_LEN);
    let map = prepare_map(&keys);

    bencher.iter(|| {
        let mut sum = 0;
        for key in &keys {
            sum += map[&format!("_{}", key)];
        }
        assert_eq!(keys.len(), sum);
    });
}

#[bench]
fn with_smart_allocations(bencher: &mut Bencher) {
    let keys = gen_keys(LEN, KEY_LEN);
    let map = prepare_map(&keys);

    bencher.iter(|| {
        let mut sum = 0;
        for key in &keys {
            let mut lookup_key = String::with_capacity(key.len() + 1);
            lookup_key.push('_');
            lookup_key += key;
            sum += map[&lookup_key];
        }
        assert_eq!(keys.len(), sum);
    });
}

#[bench]
fn without_allocations(bencher: &mut Bencher) {
    let keys = gen_keys(LEN, KEY_LEN);
    let map = prepare_map(&keys);

    bencher.iter(|| {
        let mut sum = 0;
        for key in &keys {
            sum += map[&ShortKey(&key) as &Key];
        }
        assert_eq!(keys.len(), sum);
    });
}

// Helpers
type PredictableHasher = BuildHasherDefault<DefaultHasher>; // No randomness!

fn gen_keys(len: usize, key_len: usize) -> Vec<String> {
    let mut vec = Vec::new();
    let mut rng = StdRng::from_seed([0; 32]); // No randomness!
    for _ in 0..len {
        let key = (0..key_len)
            .map(|_| rng.sample(rand::distributions::Alphanumeric))
            .collect::<String>();
        vec.push(key);
    }
    vec
}

fn prepare_map(keys: &[String]) -> HashMap<String, usize, PredictableHasher> {
    let mut map: HashMap<String, usize, PredictableHasher> = HashMap::default();
    for key in keys {
        map.insert(format!("_{}", key), 1);
        map.insert(key.clone(), 1);
    }
    map
}

const LEN: usize = 1000;
const KEY_LEN: usize = 10;
