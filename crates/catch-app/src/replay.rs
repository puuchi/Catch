//! Bounded replay responses. Evicting a response keeps its receipt so a retry
//! cannot re-execute a committed command while the request is still retained.
use crate::{Error, Result, Snapshot};
use std::collections::VecDeque;
const MAX_REQUESTS: usize = 512;
const PAYLOAD_BUDGET: usize = 4 * 1024 * 1024;
struct Receipt {
    id: Box<str>,
    fingerprint: Box<[u8]>,
    response: Option<Box<[u8]>>,
}
impl Receipt {
    fn bytes(&self) -> usize {
        self.id.len() + self.fingerprint.len() + self.response.as_ref().map_or(0, |v| v.len())
    }
}
pub(crate) struct ReplayCache {
    entries: VecDeque<Receipt>,
    bytes: usize,
    budget: usize,
}
impl Default for ReplayCache {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            bytes: 0,
            budget: PAYLOAD_BUDGET,
        }
    }
}
impl ReplayCache {
    pub fn lookup(&self, id: &str, fingerprint: &[u8]) -> Option<Result<Snapshot>> {
        let receipt = self.entries.iter().find(|entry| entry.id.as_ref() == id)?;
        Some(if receipt.fingerprint.as_ref() != fingerprint {
            Err(Error::conflict("重复请求参数不一致"))
        } else if let Some(bytes) = &receipt.response {
            serde_json::from_slice(bytes).map_err(|error| Error::storage(error.to_string()))
        } else {
            Err(Error::conflict("此操作已处理，结果已移出缓存，请刷新状态"))
        })
    }
    pub fn record(&mut self, id: String, fingerprint: Vec<u8>, snapshot: &Snapshot) {
        let receipt = Receipt {
            id: id.into_boxed_str(),
            fingerprint: fingerprint.into_boxed_slice(),
            response: serde_json::to_vec(snapshot).ok().map(Vec::into_boxed_slice),
        };
        self.bytes += receipt.bytes();
        self.entries.push_back(receipt);
        while self.entries.len() > MAX_REQUESTS {
            self.bytes -= self.entries.pop_front().unwrap().bytes();
        }
        // Drop oldest response bodies before request receipts. The budget is
        // payload bytes; container metadata and transient encoding are separate.
        for receipt in &mut self.entries {
            if self.bytes <= self.budget {
                break;
            }
            if let Some(response) = receipt.response.take() {
                self.bytes -= response.len();
            }
        }
        while self.bytes > self.budget {
            self.bytes -= self.entries.pop_front().unwrap().bytes();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use catch_core::{growth, recurrence, todos, tracking};
    fn snapshot() -> Snapshot {
        Snapshot {
            tracking: tracking::Document::default(),
            todos: todos::Document::default(),
            growth: growth::Document::default(),
            recurrences: recurrence::Document::default(),
            tracking_notice: None,
            storage_blocked: false,
        }
    }
    #[test]
    fn retained_replays_are_exact_and_evicted_bodies_are_not_cache_misses() {
        let mut cache = ReplayCache {
            budget: 2048,
            ..Default::default()
        };
        let original = snapshot();
        cache.record("first".into(), vec![1], &original);
        let replay = cache.lookup("first", &[1]).unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(replay).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        for index in 0..20 {
            cache.record(format!("request-{index}"), vec![2], &original);
        }
        assert!(cache.bytes <= cache.budget);
        assert!(cache.lookup("first", &[1]).unwrap().is_err());
        assert!(cache.lookup("first", &[9]).unwrap().is_err());
        assert!(cache.lookup("never-seen", &[1]).is_none());
    }
    #[test]
    fn large_history_and_request_payloads_cannot_bypass_either_limit() {
        let mut cache = ReplayCache {
            budget: 4096,
            ..Default::default()
        };
        let mut history = snapshot();
        history.tracking_notice = Some("合成大历史".repeat(10_000));
        cache.record("large".into(), vec![0], &history);
        assert!(cache.bytes <= cache.budget);
        assert!(cache.lookup("large", &[0]).unwrap().is_err());
        for index in 0..600 {
            cache.record(index.to_string(), vec![0; 100], &history);
        }
        assert!(cache.entries.len() <= MAX_REQUESTS);
        assert!(cache.bytes <= cache.budget);
        assert!(cache.lookup("599", &[0; 100]).unwrap().is_err());
    }
}
