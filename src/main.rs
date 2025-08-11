use std::io::{self, Error};
use configuration::{ProxyConfiguration, ProxyMode};
use log::{info,error};
use proxy_target::{DnsResolver, SimpleCachingDnsResolver};
use tokio::{net::{self, TcpListener, TcpSocket}, sync::watch::error};

mod configuration;
mod relay;
mod proxy_target;


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
