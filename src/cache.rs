use std::{
    collections::HashMap,
    hash::Hash,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct Cache<K, V> {
    inner: HashMap<K, Entry<V>>,
    timeout: u64,
}

struct Entry<V> {
    timestamp: AtomicU64,
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

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key).map(|e| {
            e.timestamp.store(Self::now(), Ordering::SeqCst);
            &e.value
        })
    }

    pub fn shrink(&mut self) {
        self.inner.retain(|_, e| {
            if Self::now().saturating_sub(e.timestamp.load(Ordering::SeqCst)) < self.timeout {
                true
            } else {
                false
            }
        });
    }

    pub fn set(&mut self, key: K, value: V) {
        self.shrink();

        let entry = Entry {
            timestamp: AtomicU64::new(Self::now()),
            value,
        };

        self.inner.insert(key, entry);
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }
}
