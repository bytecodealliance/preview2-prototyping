#![allow(unused_variables)]

use crate::{
    wasi::ip_name_lookup::{self, ResolveAddressStream},
    wasi::network::{Error, IpAddress, IpAddressFamily, Network},
    WasiSocketsView,
};
use wasi_common::wasi::poll::Pollable;

#[async_trait::async_trait]
impl<T: WasiSocketsView> ip_name_lookup::Host for T {
    async fn resolve_addresses(
        &mut self,
        network: Network,
        name: String,
        address_family: Option<IpAddressFamily>,
        include_unavailable: bool,
    ) -> anyhow::Result<Result<ResolveAddressStream, Error>> {
        todo!()
    }

    async fn resolve_next_address(
        &mut self,
        stream: ResolveAddressStream,
    ) -> anyhow::Result<Result<Option<IpAddress>, Error>> {
        todo!()
    }

    async fn drop_resolve_address_stream(
        &mut self,
        stream: ResolveAddressStream,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn non_blocking(
        &mut self,
        stream: ResolveAddressStream,
    ) -> anyhow::Result<Result<bool, Error>> {
        todo!()
    }

    async fn set_non_blocking(
        &mut self,
        stream: ResolveAddressStream,
        value: bool,
    ) -> anyhow::Result<Result<(), Error>> {
        todo!()
    }

    async fn subscribe(&mut self, stream: ResolveAddressStream) -> anyhow::Result<Pollable> {
        todo!()
    }
}
