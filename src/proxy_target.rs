use std::{collections::HashMap, hash::Hash, net::SocketAddr, sync::{Arc, RwLock}, time::Instant};

use humantime_serde::re::humantime::Duration;
use tokio::io;

use rand::thread_rng;

type CachedSocketAddr = (Vec<SocketAddr>,u128);

#[async_trait]
pub trait DnsResolver {
    async fn resolve(&mut self, target:&str) -> io::Result<SocketAddr>;
}

#[derive(Clone)]
pub struct SimpleCachingDnsResolver {
    cache: Arc<RwLock<HashMap<String, CachedSocketAddr>>>,
    ttl:Duration,
    start_time:Instant
        
}

#[async_trait]
impl DnsResolver for SimpleCachingDnsResolver {
    async fn resolve(&mut self, target:&str) -> io::Result<SocketAddr> {
        match self.try_find(target).await {
            Some(a) => Ok(a),
            _ => Ok(self.resolve_and_cache(target).await?),
        }
    }

}

impl SimpleCachingDnsResolver {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            start_time: Instant::now(),
        }
    }
}
