use anyhow::Error;
use cap_net_ext::AddressFamily;
use cap_std::net::Pool;
use wasi_common::Table;

mod ip_name_lookup;
mod network;
mod network_impl;
mod tcp;
mod tcp_socket;
mod udp;
pub mod wasi;
pub use network::WasiNetwork;
pub use tcp_socket::WasiTcpSocket;

pub type NetworkCreator = Box<dyn Fn(Pool) -> Result<Box<dyn WasiNetwork>, Error> + Send + Sync>;
pub type TcpSocketCreator =
    Box<dyn Fn(AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> + Send + Sync>;

pub struct WasiSocketsCtx {
    pub pool: Pool,
    pub network_creator: NetworkCreator,
    pub tcp_socket_creator: TcpSocketCreator,
}

pub trait WasiSocketsView: Send {
    fn ctx(&mut self) -> &mut WasiSocketsCtx;
    fn table(&mut self) -> &mut Table;
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
