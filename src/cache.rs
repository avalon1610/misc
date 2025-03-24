use std::{collections::HashMap, hash::Hash, time::Instant};

pub struct Cache<K, V> {
    inner: HashMap<K, Entry<V>>,
    timeout: u64,
}

struct Entry<V> {
    timestamp: Instant,
    value: V,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash,
{
    pub fn new(secs: u64) -> Self {
        Cache {
            inner: HashMap::new(),
            timeout: secs,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.inner.get_mut(key).map(|e| {
            e.timestamp = Instant::now();
            &e.value
        })
    }

    pub fn shrink(&mut self) {
        self.inner.retain(|_, e| {
            if e.timestamp.elapsed().as_secs() < self.timeout {
                true
            } else {
                false
            }
        });
    }

    pub fn set(&mut self, key: K, value: V) {
        let now = Instant::now();
        self.shrink();

        let entry = Entry {
            timestamp: now,
            value,
        };

        self.inner.insert(key, entry);
    }
}
