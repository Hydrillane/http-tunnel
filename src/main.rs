use std::{io::{self, Error}, thread::spawn};
use configuration::{ProxyConfiguration, ProxyMode};
use http_tunnel_codec::{HttpTunnelCodec, HttpTunnelCodecBuilder, HttpTunnelTarget};
use log::{info,error};
use proxy_target::{DnsResolver, SimpleCachingDnsResolver, SimpleTcpConnector};
use tokio::{io::{AsyncRead, AsyncWrite}, net::{self, TcpListener, TcpSocket}, sync::watch::error, task};

mod configuration;
mod tunnel;
mod http_tunnel_codec;
mod relay;
mod proxy_target;
use rand::{thread_rng,Rng};
use tunnel::TunnelCtxBuilder;


async fn main() -> io::Result<()> {

    let proxy_configuration = ProxyConfiguration::from_command_line().map_err(|e| {
        println!("Failed to process parameters");
        e
    })?;

    let dns_resolver = SimpleCachingDnsResolver::new(
        proxy_configuration
        .tunnel_config
        .target_connection
        .dns_cache_ttl,
    );

    match &proxy_configuration.mode {
        ProxyMode::Http => {
            serve_plain_text(&proxy_configuration,dns_resolver).await;
        }
    }


}

async fn start_listening_tcp(config: &ProxyConfiguration) -> Result<TcpListener,Error> {
    let bind_address = &config.bind_address;

    match TcpListener::bind(&bind_address).await {
        Ok(listener) => {
            info!("Succes to bind address {bind_address}: {listener}");
            Ok(listener)
        }
        Err(e) => {
        error!("Failed to bind address {}: {}",bind_address,e);
        Err(e)
        }
    }
}

async fn serve_plain_text(proxy_configuration:ProxyConfiguration, dns_resolver:DnsResolver) -> io::Result<()> {
    let listener = start_listening_tcp(&config).await?;

    loop {
        let socket = listener.accept().await;

        let dns_resolver_ref = dns_resolver.clone();

        match socket{
            Ok((stream,_)) => {
                stream.nodelay().unwrap_or_default();
                let config = config.clone();
                // handle accepted connections asynchronously
                tokio::spawn(async move {
                    tunnel_stream(&config, client_connection, dns_resolver)

                })  

            }
        }

    }

}

async fn start_listening_tcp(config:ProxyConfiguration) -> Result<TcpListener,Error> {
    let address = &config.bind_address;
    
    match TcpListener::bind(address).await {
        Ok(s) => {
            info!("Serving request on: {bind_address}");
            Ok(s)
        },
        Err(e) => {
            // error! is the highest priority on log crate and trace! is the lowest!
            error!("Failed to bind to address: {}");
            Err(e)
        }
    }
}

async fn tunnel_stream<C: AsyncRead + AsyncWrite + Send + Unpin + 'static>(
    config: &ProxyConfiguration,
    client_connection:C,
    dns_resolver:DnsResolver
) -> io::Result<()> {
    let ctx = TunnelCtxBuilder::default()
        .id(thread_rng().r#gen::<u128>())
        .build()
        .expect("Tunnelctxbuilder: failed");

    let codex: HttpTunnelCodec = HttpTunnelCodecBuilder::default()
        .tunnel_ctx(ctx)
        .enabled_targets(
            config.tunnel_config.target_connection.allowed_targets.clone()
        )
        .build()
        .expect("HttpTunnelCodecBuilder failed");

    let connector: SimpleTcpConnector<HttpTunnelTarget, DnsResolver>

}
