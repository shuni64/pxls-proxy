use std::{
    net::{Ipv6Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use clap::{ArgAction, Parser};
use color_eyre::{
    Result,
    eyre::{ContextCompat, eyre},
};
use log::info;
use pingora::{
    Error, ErrorType,
    prelude::HttpPeer,
    server::{RunArgs, Server},
    upstreams::peer::PeerOptions,
};
use pingora_proxy::{ProxyHttp, Session, http_proxy_service};
use sha2::{Digest, Sha256};

#[derive(Debug, Parser)]
struct Opt {
    #[clap(short, long)]
    src_prefix: Ipv6Net,

    #[clap(long)]
    target_name: String,

    #[clap(long)]
    target_port: u16,

    #[clap(long, action = ArgAction::Set)]
    target_tls: bool,
}

#[derive(Debug, Clone)]
struct Ipv6Net {
    addr: Ipv6Addr,
    prefix_len: u8,
}

impl Ipv6Net {
    pub fn from_addr_prefix(addr: Ipv6Addr, prefix_len: u8) -> Result<Self> {
        if prefix_len > 128 {
            return Err(eyre!("bad prefix length"));
        }
        if (addr.to_bits() << prefix_len).count_ones() > 0 {
            return Err(eyre!("address not matching prefix length"));
        }
        Ok(Self { addr, prefix_len })
    }

    // mask for the upper prefix_len bits
    #[inline(always)]
    pub fn subnet_mask(&self) -> u128 {
        !((1u128 << (128 - self.prefix_len)).wrapping_sub(1))
    }

    pub fn contains_addr(&self, addr: Ipv6Addr) -> bool {
        addr.to_bits() & self.subnet_mask() == self.addr.to_bits()
    }
}

impl FromStr for Ipv6Net {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (addr_str, prefix_str) = s.split_once("/").wrap_err("missing prefix length")?;
        let addr = Ipv6Addr::from_str(addr_str)?;
        let prefix_len = u8::from_str(prefix_str)?;
        Self::from_addr_prefix(addr, prefix_len)
    }
}

#[derive(Debug, Clone)]
pub struct SnatGateway {
    src_prefix: Ipv6Net,
    target_name: String,
    target_port: u16,
    target_tls: bool,
}

#[async_trait]
impl ProxyHttp for SnatGateway {
    type CTX = SnatGateway;

    fn new_ctx(&self) -> Self::CTX {
        self.clone()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let Some(client_addr) = session.client_addr() else {
            return Err(Error::new(ErrorType::Custom("client address unavailable")));
        };
        let Some(client_addr) = client_addr.as_inet() else {
            return Err(Error::new(ErrorType::Custom("client address unavailable")));
        };

        // calculate hash from client IP address
        let mut hasher = Sha256::new();
        match client_addr {
            SocketAddr::V4(client_addr) => {
                hasher.update([4]);
                hasher.update(client_addr.ip().octets());
            }
            SocketAddr::V6(client_addr) => {
                hasher.update([6]);
                hasher.update(client_addr.ip().octets());
            }
        }
        let hash = hasher.finalize();

        // first truncate hash to 128-bit integer for easier processing
        let hash_u128 = u128::from_be_bytes(hash[..16].try_into().unwrap());

        // then append to source prefix
        // effectively truncates the hash further to match the prefix
        let subnet_mask = ctx.src_prefix.subnet_mask();
        let src_addr = ctx.src_prefix.addr.to_bits() | (hash_u128 & !subnet_mask);
        let src_addr = Ipv6Addr::from_bits(src_addr);

        let addr = (ctx.target_name.as_str(), ctx.target_port);
        let addr2 = (ctx.target_name.clone(), ctx.target_port);

        let mut options = PeerOptions::new();

        options.upstream_tcp_sock_tweak_hook = Some(Arc::new(move |socket| {
            // enable IP_FREEBIND
            // required for IPv6 in combination with Any-IP
            nix::sys::socket::setsockopt(&socket, nix::sys::socket::sockopt::IpFreebind, &true)
                .map_err(|_| Error::new(ErrorType::SocketError))?;

            // bind to IP specified by header
            socket
                .bind((src_addr, 0).into())
                .map_err(|_| Error::new(ErrorType::BindError))?;

            let Ok(local_addr) = socket.local_addr() else {
                return Err(Error::new(ErrorType::SocketError));
            };
            let SocketAddr::V6(local_addr) = local_addr else {
                return Err(Error::new(ErrorType::SocketError));
            };
            info!("connecting to {addr2:?} as {local_addr:?}");

            Ok(())
        }));

        // must never reuse connections
        // (can't rebind existing socket to new IP, silently fails)
        options.idle_timeout = Some(Duration::ZERO);

        let mut peer = Box::new(HttpPeer::new(
            addr,
            ctx.target_tls,
            ctx.target_name.to_string(),
        ));
        peer.options = options;
        Ok(peer)
    }
}

fn main() -> Result<()> {
    let opt = Opt::parse();

    env_logger::init();

    info!("Starting Pingora server");

    let mut server = Server::new(None).unwrap();

    let mut gateway = http_proxy_service(
        &server.configuration,
        SnatGateway {
            src_prefix: opt.src_prefix,
            target_name: opt.target_name,
            target_port: opt.target_port,
            target_tls: opt.target_tls,
        },
    );
    gateway.add_tcp("127.0.0.1:6188");

    server.add_service(gateway);

    server.bootstrap();
    server.run(RunArgs::default());

    info!("Exiting");

    Ok(())
}
