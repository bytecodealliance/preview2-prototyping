pub mod net;

pub use cap_net_ext::AddressFamily;
pub use cap_std::net::TcpListener;
pub use cap_std::AmbientAuthority;

use crate::net::{Network, TcpSocket};
use anyhow::Error;
use cap_std::net::{Ipv4Addr, Ipv6Addr, Pool};
use ipnet::IpNet;
use wasmtime_wasi_sockets::{WasiNetwork, WasiSocketsCtx, WasiTcpSocket};

pub struct WasiSocketsCtxBuilder {
    pool: Pool,
}

impl WasiSocketsCtxBuilder {
    pub fn new() -> Self {
        Self { pool: Pool::new() }
    }

    pub fn inherit_network(mut self, ambient_authority: AmbientAuthority) -> Self {
        self.pool.insert_ip_net_port_any(
            IpNet::new(Ipv4Addr::UNSPECIFIED.into(), 0).unwrap(),
            ambient_authority,
        );
        self.pool.insert_ip_net_port_any(
            IpNet::new(Ipv6Addr::UNSPECIFIED.into(), 0).unwrap(),
            ambient_authority,
        );
        self
    }

    /* FIXME: idk how to translate this idiom because i don't have any tests checked in showing its
     * use. we cant allocate the fd until build().
    pub fn preopened_listener(mut self, fd: u32, listener: impl Into<TcpSocket>) -> Self {
        let listener: TcpSocket = listener.into();
        let listener: Box<dyn WasiTcpSocket> = Box::new(TcpSocket::from(listener));

        self.0.insert_listener(fd, listener);
        self
    }
    */

    pub fn build(self) -> WasiSocketsCtx {
        WasiSocketsCtx::new(
            self.pool,
            Box::new(create_network),
            Box::new(create_tcp_socket),
        )
    }
}

fn create_network(pool: Pool) -> Result<Box<dyn WasiNetwork>, Error> {
    let network: Box<dyn WasiNetwork> = Box::new(Network::new(pool));
    Ok(network)
}

fn create_tcp_socket(address_family: AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> {
    let socket: Box<dyn WasiTcpSocket> = Box::new(TcpSocket::new(address_family)?);
    Ok(socket)
}
