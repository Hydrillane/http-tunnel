use std::{collections::HashMap, hash::Hash, marker::PhantomData, net::SocketAddr, sync::{Arc, RwLock}, time::Instant};
use derive_builder;

use humantime_serde::re::humantime::Duration;
use tokio::io;

use rand::thread_rng;

use crate::tunnel::TunnelCtx;

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

#[derive(Clone,Builder)]
pub struct SimpleTcpConnector<D, R:DnsResolver> {
    connect_timeout: Duration,
    tunnel_ctx: TunnelCtx,
    dns_resolver:R,
    #[builder(setter(skip))]
    _phantom_target:PhantomData<D>, 
}


#[derive(Eq,PartialEq,Debug,Clone)]
pub struct Nugget {
    data:Arc<Vec<u8>>
}

impl<D,R> SimpleTcpConnector<D,R>
where 
    R:DnsResolver,
{
    pub fn new(dns_resolver:R,connection_timeout:Duration,tunnel_ctx_:TunnelCtx) -> Self  {
        Self {
            dns_resolver,
            connect_timeout,
            tunnel_ctx,
            _phantom_target:PhantomData,
        }

    }
}
