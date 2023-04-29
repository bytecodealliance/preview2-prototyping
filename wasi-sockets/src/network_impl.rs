use crate::{
    wasi::instance_network,
    wasi::network::{self, Network},
    WasiNetwork, WasiSocketsView,
};
use cap_std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub(crate) fn convert(_error: wasi_common::Error) -> anyhow::Error {
    todo!("convert wasi-common Error to wasi_network::Error")
}

#[async_trait::async_trait]
impl<T: WasiSocketsView> network::Host for T {
    async fn drop_network(&mut self, this: Network) -> anyhow::Result<()> {
        let table = self.table();
        if !table.delete::<Box<dyn WasiNetwork>>(this).is_ok() {
            anyhow::bail!("{this} is not a network");
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T: WasiSocketsView> instance_network::Host for T {
    async fn instance_network(&mut self) -> anyhow::Result<Network> {
        let ctx = self.ctx();
        let network = (ctx.network_creator)(ctx.pool.clone())?;
        let table = self.table();
        let network = table.push(Box::new(network)).map_err(convert)?;
        Ok(network)
    }
}

impl From<SocketAddr> for network::IpSocketAddress {
    fn from(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4) => Self::Ipv4(v4.into()),
            SocketAddr::V6(v6) => Self::Ipv6(v6.into()),
        }
    }
}

impl From<SocketAddrV4> for network::Ipv4SocketAddress {
    fn from(addr: SocketAddrV4) -> Self {
        Self {
            address: MyIpv4Addr::from(addr.ip()).0,
            port: addr.port(),
        }
    }
}

impl From<SocketAddrV6> for network::Ipv6SocketAddress {
    fn from(addr: SocketAddrV6) -> Self {
        Self {
            address: MyIpv6Addr::from(addr.ip()).0,
            port: addr.port(),
            flow_info: addr.flowinfo(),
            scope_id: addr.scope_id(),
        }
    }
}

// Newtypes to guide conversions.
struct MyIpv4Addr((u8, u8, u8, u8));
struct MyIpv6Addr((u16, u16, u16, u16, u16, u16, u16, u16));

impl From<&Ipv4Addr> for MyIpv4Addr {
    fn from(addr: &Ipv4Addr) -> Self {
        let octets = addr.octets();
        Self((octets[0], octets[1], octets[2], octets[3]))
    }
}

impl From<&Ipv6Addr> for MyIpv6Addr {
    fn from(addr: &Ipv6Addr) -> Self {
        let segments = addr.segments();
        Self((
            segments[0],
            segments[1],
            segments[2],
            segments[3],
            segments[4],
            segments[5],
            segments[6],
            segments[7],
        ))
    }
}
