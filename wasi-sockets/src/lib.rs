use anyhow::Error;
use cap_net_ext::AddressFamily;
use cap_std::ambient_authority;
use cap_std::net::Pool;
use wasi_common::Table;

mod ip_name_lookup;
mod network;
mod network_impl;
mod tcp;
mod tcp_socket;
mod udp;
mod udp_socket;
pub mod wasi;
pub use network::WasiNetwork;
pub use tcp_socket::WasiTcpSocket;
pub use udp_socket::{RiFlags, RoFlags, WasiUdpSocket};

pub type NetworkCreator = Box<dyn Fn(Pool) -> Result<Box<dyn WasiNetwork>, Error> + Send + Sync>;
pub type TcpSocketCreator =
    Box<dyn Fn(AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> + Send + Sync>;

pub struct WasiSocketsCtx {
    pool: Pool,
    network_creator: NetworkCreator,
    tcp_socket_creator: TcpSocketCreator,
}

impl WasiSocketsCtx {
    pub fn new(
        pool: Pool,
        network_creator: NetworkCreator,
        tcp_socket_creator: TcpSocketCreator,
    ) -> Self {
        Self {
            pool,
            network_creator,
            tcp_socket_creator,
        }
    }

    /// Add network addresses to the pool.
    pub fn insert_addr<A: cap_std::net::ToSocketAddrs>(&mut self, addrs: A) -> std::io::Result<()> {
        self.pool.insert(addrs, ambient_authority())
    }

    /// Add a specific [`cap_std::net::SocketAddr`] to the pool.
    pub fn insert_socket_addr(&mut self, addr: cap_std::net::SocketAddr) {
        self.pool.insert_socket_addr(addr, ambient_authority());
    }

    /// Add a range of network addresses, accepting any port, to the pool.
    ///
    /// Unlike `insert_ip_net`, this function grants access to any requested port.
    pub fn insert_ip_net_port_any(&mut self, ip_net: ipnet::IpNet) {
        self.pool
            .insert_ip_net_port_any(ip_net, ambient_authority())
    }

    /// Add a range of network addresses, accepting a range of ports, to
    /// per-instance networks.
    ///
    /// This grants access to the port range starting at `ports_start` and, if
    /// `ports_end` is provided, ending before `ports_end`.
    pub fn insert_ip_net_port_range(
        &mut self,
        ip_net: ipnet::IpNet,
        ports_start: u16,
        ports_end: Option<u16>,
    ) {
        self.pool
            .insert_ip_net_port_range(ip_net, ports_start, ports_end, ambient_authority())
    }

    /// Add a range of network addresses with a specific port to the pool.
    pub fn insert_ip_net(&mut self, ip_net: ipnet::IpNet, port: u16) {
        self.pool.insert_ip_net(ip_net, port, ambient_authority())
    }
}

pub trait WasiSocketsView: Send {
    fn table(&self) -> &Table;
    fn table_mut(&mut self) -> &mut Table;
    fn ctx(&self) -> &WasiSocketsCtx;
    fn ctx_mut(&mut self) -> &mut WasiSocketsCtx;
}

pub fn add_to_linker<T: WasiSocketsView>(
    l: &mut wasmtime::component::Linker<T>,
) -> anyhow::Result<()> {
    crate::wasi::tcp::add_to_linker(l, |t| t)?;
    crate::wasi::tcp_create_socket::add_to_linker(l, |t| t)?;
    crate::wasi::udp::add_to_linker(l, |t| t)?;
    crate::wasi::udp_create_socket::add_to_linker(l, |t| t)?;
    crate::wasi::ip_name_lookup::add_to_linker(l, |t| t)?;
    crate::wasi::instance_network::add_to_linker(l, |t| t)?;
    crate::wasi::network::add_to_linker(l, |t| t)?;
    Ok(())
}
