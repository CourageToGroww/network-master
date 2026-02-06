use std::net::IpAddr;
use std::sync::Mutex;

use lru::LruCache;

pub struct DnsResolver {
    cache: Mutex<LruCache<IpAddr, Option<String>>>,
}

impl DnsResolver {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap_or(std::num::NonZeroUsize::new(1024).unwrap()),
            )),
        }
    }

    pub async fn reverse_lookup(&self, ip: IpAddr) -> Option<String> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&ip) {
                return cached.clone();
            }
        }

        // Perform reverse DNS lookup
        let result = tokio::task::spawn_blocking(move || {
            dns_lookup::lookup_addr(&ip).ok()
        })
        .await
        .ok()
        .flatten();

        // Cache the result
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(ip, result.clone());
        }

        result
    }
}
