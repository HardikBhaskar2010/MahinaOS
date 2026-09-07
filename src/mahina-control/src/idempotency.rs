use crate::protocol::Response;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

const DEFAULT_CAPACITY: usize = 1024;

/// In-memory idempotency cache protecting mutating requests against duplicate execution
pub struct IdempotencyStore {
    inner: Mutex<StoreInner>,
}

struct StoreInner {
    capacity: usize,
    entries: HashMap<String, Response>,
    queue: VecDeque<String>,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(StoreInner {
                capacity,
                entries: HashMap::with_capacity(capacity),
                queue: VecDeque::with_capacity(capacity),
            }),
        }
    }

    /// Check if a response is already cached for this request_id
    pub fn get(&self, request_id: &str) -> Option<Response> {
        let guard = self.inner.lock().ok()?;
        guard.entries.get(request_id).cloned()
    }

    /// Cache a response for a given request_id, evicting oldest if full
    pub fn insert(&self, request_id: String, response: Response) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.entries.contains_key(&request_id) {
                guard.entries.insert(request_id, response);
                return;
            }

            if guard.entries.len() >= guard.capacity {
                if let Some(oldest) = guard.queue.pop_front() {
                    guard.entries.remove(&oldest);
                }
            }

            guard.queue.push_back(request_id.clone());
            guard.entries.insert(request_id, response);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.entries.clear();
            guard.queue.clear();
        }
    }
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_cache_hit_and_miss() {
        let store = IdempotencyStore::new();
        assert_eq!(store.get("req-1"), None);

        let resp = Response::success(1, serde_json::json!({"status": "rebooting"}));
        store.insert("req-1".to_string(), resp.clone());

        let cached = store.get("req-1").expect("cache hit expected");
        assert_eq!(cached, resp);
    }

    #[test]
    fn test_idempotency_eviction() {
        let store = IdempotencyStore::with_capacity(2);
        let resp = Response::success(1, serde_json::json!({}));

        store.insert("req-1".to_string(), resp.clone());
        store.insert("req-2".to_string(), resp.clone());
        store.insert("req-3".to_string(), resp.clone()); // Should evict req-1

        assert_eq!(store.get("req-1"), None);
        assert!(store.get("req-2").is_some());
        assert!(store.get("req-3").is_some());
    }
}
