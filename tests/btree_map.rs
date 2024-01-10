use std::{borrow::Borrow, collections::HashSet};

use fast_btree::btree_map::DefaultBTreeMap;
use rand::{seq::SliceRandom, thread_rng, Rng};

#[test]
fn btree_map_works() {
    let mut tree = Box::new(DefaultBTreeMap::<i32, i32>::new());

    for i in 0..1000 {
        tree.put(i, i + 1);
    }

    for i in 0..1000 {
        assert_eq!(tree.get(&i), Some(&(i + 1)));
    }

    assert_eq!(tree.get(&12), Some(&13));
    tree.remove(&12);
    assert!(tree.get(&12).is_none());
    tree.put(12, 24);
    assert_eq!(tree.get(&12), Some(&24));

    for i in 0..1000 {
        if i == 12 {
            assert_eq!(tree.get(&i), Some(&24));
        } else {
            assert_eq!(tree.get(&i), Some(&(i + 1)));
        }
    }
}

#[test]
fn works_on_pointer_types() {
    let mut tree = Box::new(DefaultBTreeMap::<String, String>::new());
    assert_eq!(tree.get(&"test".into()), None);
    tree.put("test".into(), "test2".into());
    assert_eq!(tree.get(&"test".into()), Some(&("test2".to_string())));
    for i in 0..100 {
        tree.put(i.to_string(), (i + 1).to_string());
    }
    for i in 0..100 {
        assert_eq!(
            tree.get(i.to_string().borrow()),
            Some((i + 1).to_string().borrow()),
        );
    }
}

#[test]
fn random_op_test() {
    let mut tree = Box::new(DefaultBTreeMap::<i32, i32>::new());

    let n = 50000;

    let mut rng = thread_rng();

    let mut keys = HashSet::new();
    while keys.len() < n {
        keys.insert(rng.gen::<u16>() as i32);
    }
    let mut keys: Vec<_> = keys.into_iter().collect();

    for &key in keys.iter() {
        tree.put(key, key + 1);
    }

    for &key in keys.iter() {
        assert_eq!(tree.get(&key), Some(&(key + 1)));
    }

    keys.shuffle(&mut rng);
    let removed_keys = keys.split_off(n / 2);
    for &key in removed_keys.iter() {
        tree.remove(&key);
    }

    for &key in removed_keys.iter() {
        assert!(tree.get(&key).is_none());
    }

    for &key in keys.iter() {
        assert_eq!(tree.get(&key), Some(&(key + 1)));
    }
}
