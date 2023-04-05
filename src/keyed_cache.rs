use std::{cell::Cell, collections::HashMap, hash::Hash};

pub struct KeyedCache<K, V>
where
    K: Eq + Hash + Clone,
{
    storage: HashMap<K, (usize, V)>,
    generation: usize,
    queries: Cell<usize>,
    misses: Cell<usize>,
}

impl<K, V> KeyedCache<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            generation: 0,
            queries: Cell::new(0),
            misses: Cell::new(0),
        }
    }

    pub fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, default: F) -> &V {
        self.queries.set(self.queries.get() + 1);

        let (_, v) = self
            .storage
            .entry(key.clone())
            .and_modify(|(age, _)| *age = self.generation)
            .or_insert_with(|| {
                self.misses.set(self.misses.get() + 1);
                (self.generation, default())
            });
        v
    }

    pub fn age(&mut self) {
        self.generation += 1;

        self.storage
            .retain(|_, (age, _)| *age + 60 > self.generation);

        let queries = self.queries.get();
        let misses = self.misses.get();
        let _hits = queries - misses;

        // debug!("cache stats: {hits} hits, {misses} misses");

        self.queries.set(0);
        self.misses.set(0);
    }
}
