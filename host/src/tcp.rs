#![allow(unused_variables)]

use crate::{
    wasi_tcp::{
        self, ConnectionFlags, Errno, IoSize, IpSocketAddress, ListenerFlags, Network, Size,
        TcpConnection, TcpListener, WasiTcp,
    },
    HostResult, WasiCtx, WasiStream,
};

#[async_trait::async_trait]
impl WasiTcp for WasiCtx {
    async fn listen(
        &mut self,
        network: Network,
        address: IpSocketAddress,
        backlog: Option<Size>,
        flags: ListenerFlags,
    ) -> HostResult<TcpListener, Errno> {
        todo!()
    }

    async fn accept(
        &mut self,
        listener: TcpListener,
        flags: ConnectionFlags,
    ) -> HostResult<(TcpConnection, IpSocketAddress), Errno> {
        todo!()
    }

    async fn connect(
        &mut self,
        network: Network,
        local_address: IpSocketAddress,
        remote_address: IpSocketAddress,
        flags: ConnectionFlags,
    ) -> HostResult<TcpConnection, Errno> {
        todo!()
    }

    async fn send(&mut self, connection: TcpConnection, bytes: Vec<u8>) -> HostResult<Size, Errno> {
        todo!()
    }

    async fn receive(
        &mut self,
        connection: TcpConnection,
        length: Size,
    ) -> HostResult<(Vec<u8>, bool), Errno> {
        todo!()
    }

    async fn get_listener_local_address(
        &mut self,
        listener: TcpListener,
    ) -> HostResult<IpSocketAddress, Errno> {
        todo!()
    }

    async fn get_tcp_connection_local_address(
        &mut self,
        connection: TcpConnection,
    ) -> HostResult<IpSocketAddress, Errno> {
        todo!()
    }

    async fn get_remote_address(
        &mut self,
        connection: TcpConnection,
    ) -> HostResult<IpSocketAddress, Errno> {
        todo!()
    }

    async fn get_flags(&mut self, connection: TcpConnection) -> HostResult<ConnectionFlags, Errno> {
        todo!()
    }

    async fn set_flags(
        &mut self,
        connection: TcpConnection,
        flags: ConnectionFlags,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn get_receive_buffer_size(
        &mut self,
        connection: TcpConnection,
    ) -> HostResult<Size, Errno> {
        todo!()
    }

    async fn set_receive_buffer_size(
        &mut self,
        connection: TcpConnection,
        value: Size,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn get_send_buffer_size(&mut self, connection: TcpConnection) -> HostResult<Size, Errno> {
        todo!()
    }

    async fn set_send_buffer_size(
        &mut self,
        connection: TcpConnection,
        value: Size,
    ) -> HostResult<(), Errno> {
        todo!()
    }

    async fn bytes_readable(&mut self, socket: TcpConnection) -> HostResult<(IoSize, bool), Errno> {
        drop(socket);
        todo!()
    }

    async fn bytes_writable(&mut self, socket: TcpConnection) -> HostResult<(IoSize, bool), Errno> {
        drop(socket);
        todo!()
    }

    async fn read_via_stream(
        &mut self,
        fd: wasi_tcp::TcpConnection,
    ) -> HostResult<WasiStream, wasi_tcp::Errno> {
        todo!()
    }

    async fn write_via_stream(
        &mut self,
        fd: wasi_tcp::TcpConnection,
    ) -> HostResult<WasiStream, wasi_tcp::Errno> {
        todo!()
    }
}
