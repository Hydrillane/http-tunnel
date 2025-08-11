use std::fs::File;
use std::time::Duration;
use std::io::{Error, ErrorKind, Read}; 

pub mod relay;

use crate::relay::{RelayPolicy,NO_BANDWITH_LIMIT,NO_TIMEOUT};
use clap::error::Error as ClapError;
use log::{error,info};
use native_tls::Identity;
use serde::Deserialize;
use clap::Parser;
use clap::Subcommand;
use clap::Args;
use serde::Serialize;
use tokio::sync::oneshot::error;
use derive_builder::Builder;
use regex::Regex;

use tokio::io;

#[derive(Deserialize,Clone)]
pub struct TargetConnectionConfig {
        #[serde(with = "humantime_serde")]
    pub dns_cache_ttl:Duration,
        #[serde(with = "serde_regex")]
    pub allowed_targets: Regex,
        #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,
    pub relay_policy:RelayPolicy,

}

#[derive(Args,Debug)]
#[command(about = " Run the tunnel in HTTPS mode", long_about = None)]
#[command(author = "Billy", version="0.1.0", long_about = None)]
#[command(propagate_version = true)]
struct HttpOptions{
    //pkcs12 filename
    #[arg(long)]
    pk:String,
    #[arg(long)]
    password:String,
}

#[derive(Args,Debug)]
#[command(about =" Run the tunnel in TCP mode", long_about = None)]
#[command(author = "Billy", version="0.1.0", long_about = None)]
#[command(propagate_version = true)]
struct TcpOptions{
    destination:String,
}

#[derive(Args,Debug)]
#[command(about =" Run the tunnel in HTTPS mode", long_about = None)]
#[command(author = "Billy", version="0.1.0", long_about = None)]
#[command(propagate_version = true)]
struct HttpsOptions {

    #[arg(long)]
    pk:String,

    #[arg(long)]
    password:String,
}

#[derive(Subcommand,Debug)]
enum Command {
    Http(HttpOptions),
    Https(HttpsOptions),
    Tcp(TcpOptions),
}


#[derive(Parser,Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(long)]
    // Config Files
    config:Option<String>,
    
    // bind eg. 8.8.4.4
    #[arg(long)]
    bind: String,
    #[command(subcommand)]
    command:Command
}


#[derive(Clone)]
pub enum ProxyMode {
    Http,
    Https(Identity),
    Tcp(String)
}

#[derive(Deserialize,Clone)]
pub struct ClientConnectionConfig {
    #[serde(with="humantime_serde")]
    pub initiation_timeout:Duration,
    pub relay_policy: RelayPolicy,
}


#[derive(Deserialize,Clone)]
pub struct TunnelConfig {
    pub client_connection: ClientConnectionConfig,
    pub target_connection: TargetConnectionConfig,

}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            client_connection: ClientConnectionConfig {
                initiation_timeout:NO_TIMEOUT,
                relay_policy: RelayPolicy {
                    idle_timeout:NO_TIMEOUT,
                    min_rate_bpm:NO_BANDWITH_LIMIT,
                    max_rate_bps:NO_BANDWITH_LIMIT,
                }
            },
            target_connection: TargetConnectionConfig {
                dns_cache_ttl: NO_TIMEOUT,
                allowed_targets: Regex::new(".*").expect("Bug: bad default regexp"),
                connection_timeout: NO_TIMEOUT,
                relay_policy: RelayPolicy {
                    idle_timeout:NO_TIMEOUT,
                    min_rate_bpm:0,
                    max_rate_bps:NO_BANDWITH_LIMIT,
                }

            }

        }

}
}

#[derive(Clone,Builder)]
pub struct ProxyConfiguration {
    pub mode: ProxyMode,
    pub bind_address: String,
    pub tunnel_config: TunnelConfig
}


impl ProxyConfiguration {
    pub fn from_command_line() -> io::Result<ProxyConfiguration> {
        let cli: Cli = Cli::parse();

        let config = cli.config;
        let bind_address = cli.bind;

        let mode = match cli.command {
            Command::Http(_) => {
                info!(
                    "Starting in HTTP mode: bind {}, configuration: {:?}",
                    bind_address,config
                );
                ProxyMode::Http
            },
            Command::Https(https) => {
                let pkcs12_file = https.pk.as_str();
                let password = https.password.as_str();

                let identity = ProxyConfiguration::tls_identity_from_file(&pkcs12_file, &password)?;
                info!(
                    "Starting in HTTPS mode: pksc12: {}, password: {}, bind: {}, configuration: {:?}",
                    pkcs12_file,
                    !password.is_empty(),
                    bind_address,
                    config
                );
                ProxyMode::Https(identity)
            },
            Command::Tcp(tcp) => {
                let tcp = tcp.destination;
                info!(
                    "Starting in TCP mode: destionation: {}, configuration: {:?}",
                    tcp,config
                );
                ProxyMode::Tcp(tcp)
            }
        };

        let tunnel_config = match config {
            None => TunnelConfig::default(),
            Some(config) => ProxyConfiguration::read_tunnel_config(config.as_str())?
        };

        Ok(ProxyConfigurationBuilder::default()
            .bind_address(bind_address)
            .mode(mode)
            .tunnel_config(tunnel_config)
            .build()
            .expect("ProxyConfigurationBuilder failed"))

    }

    fn tls_identity_from_file(file_path:&str,password:&str) -> io::Result<Identity> {
        let mut file = File::open(file_path).map_err(|e| {
            error!("Error opening file PKSC12 {}: {}",file_path,e);
            e
        })?;
        let mut identity = vec![];
        file.read_to_end(&mut identity).map_err(|e| {
            error!("Error reading file: {}, {}",file_path,e);
            e
        })?;
        Identity::from_pkcs12(&identity, &password).map_err(|e| {
            error!("Error authenting the Identity of file {}: {}",file_path,e);
            Error::from(ErrorKind::InvalidInput)
        })
    }

    fn read_tunnel_config(file_path:&str) -> io::Result<TunnelConfig> {
        let mut file = File::open(file_path).map_err(|e| {
            error!("Failed to open tunnel config file {}: {}", file_path,e);
            e
        })?;
        let mut yaml = vec![];

        file.read_to_end(&mut yaml).map_err(|e| {
            error!("Error reading file {}: {}",file_path,e);
            e
        })?;

        let result:TunnelConfig = serde_yaml::from_slice(&yaml).map_err(|e| {
            error!("Error parsing yaml {}: {}",file_path,e);
            Error::from(ErrorKind::InvalidInput)
        })?;
        Ok(result)
    }

}


#[cfg(test)]
mod test {
    use super::*;
    use std::io;
    use clap::Parser;
    use tempfile::NamedTempFile;
    use tokio::io;

    fn create_test_config() -> io::Result<NamedTempFile> {
        let config = r#"
        {
            "client_connection": {
                "initiation_timeout": "30s",
                "relay_policy": "AllowAll"
            },
            "target_connection": {
                "dns_cache_ttl": "60s",
                "allowed_targets": ".*",
                "connection_timeout": "10s",
                "relay_policy": "AllowAll"
            }
        }"#;
        
        let mut file = NamedTempFile::new()?;
        std::fs::write(file.path(), config)?;
        Ok(file)
    }

    #[test]
    fn test_http_mode() -> io::Result<()> {
        let args = vec![
            "test_proxy",
            "http",
            "--bind", "127.0.0.1:8080"
        ];

        let cli = Cli::parse_from(args);

        assert!(matches!(cli.command,Command::Http(_)));
        let config = ProxyConfiguration::from_command_line()?;
        assert!(matches!(config.mode,ProxyMode::Http));
        assert_eq!(config.bind_address,"127.0.0.1:8080");
        Ok(())

    }

    fn test_https_mode() -> io::Result<()> {
        let pksc_file = NamedTempFile::new()?;
        let pksc_path = pksc_file.path().to_str().unwrap();
        let args = vec![
            "test_proxy",
            "https",
            "--pk", pksc_path,
            "--password", "test123",
            "--bind", "127.0.0.1:8443"
        ];

        let cli = Cli::parse_from(args);
        assert!(matches!(cli.command,Command::Https(_)));

        let config = ProxyConfiguration::from_command_line();

        Ok(())

    }

}
