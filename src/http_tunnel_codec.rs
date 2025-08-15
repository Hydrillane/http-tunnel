use std::{char::MAX, fmt::{self, Write}, str::Bytes, sync::Arc};

use async_trait::async_trait;
use clap::error::ErrorKind;
use regex::{Regex, Split};
use tokio_util::codec::{Decoder,Encoder};
use bytes::BytesMut;
use derive_builder::Builder;

use log::debug;


use crate::{proxy_target::Nugget, tunnel::{EstablishTunnelResult, TunnelCtx}};

const MAX_HTTP_REQUEST_SIZE: usize = 16384;
const REQUEST_END_MARKER:&[u8] = b"\r\n\r\n";

pub struct HttpConnectRequest {
    uri:String,
    nugget:Option<Nugget>,
}

impl HttpConnectRequest {

    pub fn parse(http_request:&[u8]) -> Result<Self,EstablishTunnelResult> {
        HttpConnectRequest::precondition_size(http_request)?;
        HttpConnectRequest::precondition_legal_character(http_request)?;

        let as_string = String::from_utf8(http_request.to_vec()).expect("Contains only ASCII");

        let mut lines = as_string.split("\r\n");

        let request_line = HttpConnectRequest::parse_request_line(lines
            .next()
            .expect("At least a single line is present at this point")
            )?;

        let has_nugget = request_line.3;

        if has_nugget {
            Ok(
                Self {
                    uri:HttpConnectRequest::extract_host_destination(lines, lines.1)
                        .unwrap_or_else(|| request_line.1.to_string()),
                        nugget: Some(Nugget::new(&self, other))
                }
            )
        } else {
            Ok(
                Self {
                    uri: request_line.1.to_string(),
                    nugget:None
                }
            )
        }
    }

    fn extract_host_destination(lines:&mut Split<&str>, endpoint:&str) -> Option<String> {
        const HOST_HEADER:&str = "host:";

        lines
            .find(|line| line.starts_with(HOST_HEADER))
            .map(|line| line[HOST_HEADER.len()..].trim())
            .map(|host| {
                let mut host = String::from(host);
                if host.rfind(":").is_none() {
                    let default_port = if endpoint.to_ascii_lowercase().starts_with("https://") {
                        ":443"
                    } else {
                        ":80"
                    };

                    host.push_str(default_port);

                }
                host
            })
    }

    fn precondition_legal_character(http_request:&[u8]) -> Result<(),EstablishTunnelResult> {
        for c in http_request {
            match c {
            32..=126 | 9 | 10 | 13 => {}
            _ => {
                      debug!("Found illegal character in request header {}",
                          c);
                      return Err(EstablishTunnelResult::BadRequest);
                  }
        }
        };
        Ok(())
    }

    fn parse_request_line(lines:&str) -> Result<(&str,&str,&str,bool), EstablishTunnelResult> {
        let request_line =  lines.split(' ').collect::<Vec<&u8>>();
        HttpConnectRequest::precondition_well_formed(lines, &request_line)?;

        let method = request_line[0];
        let uri = request_line[1];
        let version = request_line[2];

        let has_nugget =  HttpConnectRequest::check_method(method)?;
        HttpConnectRequest::check_version(version)?;

        Ok((method,uri,version,has_nugget))

    }

    fn check_version(version:&str) -> Result<(),EstablishTunnelResult> {
        if version != "HTTP/1.1" {
            debug!("Failed Bad Version!: {}",version);
            EstablishTunnelResult::BadRequest
        } else {
            Ok(())
        }
    }

    fn precondition_well_formed(http_request:&str,http_request_slice:&[&u8]) -> Result<(),EstablishTunnelResult> {
        if http_request_slice.len() != 3 {
            debug!("http header not well formed! , {:?}",http_request);
            EstablishTunnelResult::BadRequest
        } else {
            Ok(())
        }
    }

    fn precondition_size(http_request:&[u8]) -> Result<(), EstablishTunnelResult> {
        if http_request.len() >= MAX_HTTP_REQUEST_SIZE {
            debug!("Error http request header {} , {}",
            http_request.len(),
            MAX_HTTP_REQUEST_SIZE,
            );
            EstablishTunnelResult::BadRequest
        } else {
            Ok(())
        }
    }

    #[cfg(not(feature="plain_text"))]
    fn check_method(method:&str) -> Result<bool,EstablishTunnelResult> {
        if !method=="CONNECT" {
            debug!("Warn! not CONNECT method operation not allowed! {}", method);
            Err(EstablishTunnelResult::Forbidden)
        } else {
            Ok(false)
        }
    }

    #[cfg(feature="plain_text")]
    fn check_method(method:&str) -> Result<bool,EstablishTunnelResult> {
         Ok(method!="CONNECT")
    }

}


#[derive(Clone,Builder)]
pub struct HttpTunnelCodec {
    tunnel_ctx: TunnelCtx,
    enabled_targets: Regex,
}

impl Decoder for HttpTunnelCodec {
    type Item = HttpTunnelTarget;
    type Error = EstablishTunnelResult;

    fn decode(&mut self,src:&mut BytesMut) -> Result<Option<Self::Item>,Self::Error>{
        if !got_http_request(src) {
            return Ok(None)
        } 

        match HttpConnectRequest::parse(src) {
            Ok(parsed_request) => {
                if self.enabled_targets.is_match(&parsed_request.uri) {
                    debug!("Target {} is not allowed, only allowed is {}, CTX= {}",
                        parsed_request.uri,
                        self.enabled_targets,
                        self.tunnel_ctx
                        );
                        Ok(EstablishTunnelResult::OperationNotAllowed)
                } else {
                    Ok(Some(
                            HttpTunnelTargetBuilder::default()
                            .target(parsed_request.uri)
                            .nugget(parsed_request.nugget)
                            .build()
                            .expect("HttpTunnelTargetBuilder is failed"),
                    ))
                }
            }
            Err(e) => Err(e)
            }

        }

    }

impl Encoder<EstablishTunnelResult> for HttpTunnelCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: EstablishTunnelResult, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (code,item) = match item {
            EstablishTunnelResult::Ok => (200,"OK"),
            EstablishTunnelResult::OkWithNugget => Ok(()),
            EstablishTunnelResult::BadGateway => (502, "BAD_GATEWAY"),
            EstablishTunnelResult::Forbidden => (403, "FORBIDDEN"),
            EstablishTunnelResult::BadRequest => (400, "BAD_REQUEST"),
            EstablishTunnelResult::ServerError => (500, "SERVER_ERROR"),
            EstablishTunnelResult::TooManyRequest => (429, "TOO_MANY_REQUEST"),
            EstablishTunnelResult::RequestTimeout => (408, "REQUEST_TIMEOUT"),
            EstablishTunnelResult::GatewayTimeout => (504, "GATEWAY_TIMEOUT"),
            EstablishTunnelResult::OperationNotAllowed => (405, "NOT_ALLOWED"),
        };
        dst.write_fmt(format_args!("HTTP/1.1 {} {}\r\n\r\n",code as u32,item)).map_err(|e| std::io::Error::from(std::io::ErrorKind::Other) )
    }
}



#[derive(Builder,Eq,PartialEq,Debug,Clone)]
pub struct HttpTunnelTarget {
    pub target: String,
    pub nugget: Option<Nugget>,

}

#[async_trait]
impl TunnelTarget for HttpTunnelTarget {

    type Addr = String;

    fn target_addr(&self) -> Self::Addr {
        self.target.clone()
    }

    fn has_nugget(&self) -> bool {
        self.nugget.is_some()
    }

    fn target_nugget(&self) -> &Nugget {
        self.nugget
            .as_ref()
            .expect("Cannot use this method without checing `has_nugget`")

    }

}

impl fmt::Display for HttpTunnelTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.target)
    }
}

// #[cfg(feature="plains_text")]
// fn got_http_request(buffers:&BytesMut) -> bool {
//     buffers.len() >= MAX_HTTP_REQUEST_SIZE ||
//         buffers
//         .windows(REQUEST_END_MARKER.len())
//         .find(|w| *w == REQUEST_END_MARKER)
//         .is_some()
// }

#[cfg(not(feature="plain_text"))]
fn got_http_request(buffers:&BytesMut) -> bool {
    buffers.len() >= MAX_HTTP_REQUEST_SIZE || buffers.ends_with(REQUEST_END_MARKER)
}




