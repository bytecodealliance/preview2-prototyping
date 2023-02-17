use cap_std::net::{TcpListener, TcpStream};
use io_extras::borrowed::BorrowedReadable;
#[cfg(windows)]
use io_extras::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};
use io_lifetimes::AsSocketlike;
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsSocket, BorrowedSocket};
use rustix::fd::OwnedFd;
use std::any::Any;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use system_interface::io::IoExt;
use system_interface::io::IsReadWrite;
use system_interface::io::ReadReady;
use wasi_common::{
    stream::{InputStream, OutputStream},
    tcp_socket::{SdFlags, WasiTcpSocket},
    udp_socket::{RiFlags, RoFlags, WasiUdpSocket},
    Error, ErrorExt,
};

pub struct TcpSocket(Arc<OwnedFd>);
pub struct UdpSocket(Arc<OwnedFd>);

impl TcpSocket {
    pub fn new(owned: OwnedFd) -> Self {
        Self(Arc::new(owned))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl UdpSocket {
    pub fn new(owned: OwnedFd) -> Self {
        Self(Arc::new(owned))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait::async_trait]
impl WasiTcpSocket for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn accept(
        &mut self,
        nonblocking: bool,
    ) -> Result<
        (
            Box<dyn WasiTcpSocket>,
            Box<dyn InputStream>,
            Box<dyn OutputStream>,
            SocketAddr,
        ),
        Error,
    > {
        let (connection, addr) = self.0.as_socketlike_view::<TcpListener>().accept()?;
        connection.set_nonblocking(nonblocking)?;
        let connection = TcpSocket::new(connection.into());
        let input_stream = connection.clone();
        let output_stream = connection.clone();
        Ok((
            Box::new(connection),
            Box::new(input_stream),
            Box::new(output_stream),
            addr,
        ))
    }

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
        self.0
            .as_socketlike_view::<TcpStream>()
            .set_nonblocking(flag)?;
        Ok(())
    }

    async fn sock_shutdown(&mut self, how: SdFlags) -> Result<(), Error> {
        let how = if how == SdFlags::READ | SdFlags::WRITE {
            cap_std::net::Shutdown::Both
        } else if how == SdFlags::READ {
            cap_std::net::Shutdown::Read
        } else if how == SdFlags::WRITE {
            cap_std::net::Shutdown::Write
        } else {
            return Err(Error::invalid_argument());
        };
        self.0.as_socketlike_view::<TcpStream>().shutdown(how)?;
        Ok(())
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl WasiUdpSocket for UdpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
        self.0
            .as_socketlike_view::<TcpStream>()
            .set_nonblocking(flag)?;
        Ok(())
    }

    async fn sock_recv<'a>(
        &mut self,
        ri_data: &mut [io::IoSliceMut<'a>],
        ri_flags: RiFlags,
    ) -> Result<(u64, RoFlags), Error> {
        if (ri_flags & !(RiFlags::RECV_PEEK | RiFlags::RECV_WAITALL)) != RiFlags::empty() {
            return Err(Error::not_supported());
        }

        if ri_flags.contains(RiFlags::RECV_PEEK) {
            if let Some(first) = ri_data.iter_mut().next() {
                let n = self.0.peek(first)?;
                return Ok((n as u64, RoFlags::empty()));
            } else {
                return Ok((0, RoFlags::empty()));
            }
        }

        if ri_flags.contains(RiFlags::RECV_WAITALL) {
            let n: usize = ri_data.iter().map(|buf| buf.len()).sum();
            self.0.read_exact_vectored(ri_data)?;
            return Ok((n as u64, RoFlags::empty()));
        }

        let n = self.0.read_vectored(ri_data)?;
        Ok((n as u64, RoFlags::empty()))
    }

    async fn sock_send<'a>(&mut self, si_data: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = self.0.write_vectored(si_data)?;
        Ok(n as u64)
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl InputStream for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(self.0.as_handle_or_socket())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        match Read::read(&mut &*self.as_socketlike_view::<TcpStream>(), buf) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        match Read::read_vectored(&mut &*self.as_socketlike_view::<TcpStream>(), bufs) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    #[cfg(can_vector)]
    fn is_read_vectored(&self) -> bool {
        Read::is_read_vectored(&mut &*self.as_socketlike_view::<TcpStream>())
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let num = io::copy(
            &mut io::Read::take(&*self.0.as_socketlike_view::<TcpStream>(), nelem),
            &mut io::sink(),
        )?;
        Ok((num, num < nelem))
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        let val = self.as_socketlike_view::<TcpStream>().num_ready_bytes()?;
        Ok(val)
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl OutputStream for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(self.0.as_handle_or_socket())
    }

    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let n = Write::write(&mut &*self.as_socketlike_view::<TcpStream>(), buf)?;
        Ok(n.try_into()?)
    }
    async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = Write::write_vectored(&mut &*self.as_socketlike_view::<TcpStream>(), bufs)?;
        Ok(n.try_into()?)
    }
    #[cfg(can_vector)]
    fn is_write_vectored(&self) -> bool {
        Write::is_write_vectored(&mut &*self.as_socketlike_view::<TcpStream>())
    }
    async fn splice(
        &mut self,
        src: &mut dyn InputStream,
        nelem: u64,
    ) -> Result<(u64, bool), Error> {
        if let Some(readable) = src.pollable_read() {
            let num = io::copy(
                &mut io::Read::take(BorrowedReadable::borrow(readable), nelem),
                &mut &*self.0.as_socketlike_view::<TcpStream>(),
            )?;
            Ok((num, num < nelem))
        } else {
            OutputStream::splice(self, src, nelem).await
        }
    }
    async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
        let num = io::copy(
            &mut io::Read::take(io::repeat(0), nelem),
            &mut &*self.0.as_socketlike_view::<TcpStream>(),
        )?;
        Ok(num)
    }
    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&self.0)?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[cfg(unix)]
impl AsFd for TcpSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(unix)]
impl AsFd for UdpSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(windows)]
impl AsSocket for TcpSocket {
    /// Borrows the socket.
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.0.as_socket()
    }
}

#[cfg(windows)]
impl AsHandleOrSocket for TcpSocket {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        self.0.as_handle_or_socket()
    }
}
#[cfg(windows)]
impl AsSocket for UdpSocket {
    /// Borrows the socket.
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.0.as_socket()
    }
}

#[cfg(windows)]
impl AsHandleOrSocket for UdpSocket {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        self.0.as_handle_or_socket()
    }
}

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`wasi_common::WasiFile::get_fdflags`].
pub fn is_read_write<Socketlike: AsSocketlike>(f: Socketlike) -> io::Result<(bool, bool)> {
    // On Unix-family platforms, we have an `IsReadWrite` impl.
    #[cfg(not(windows))]
    {
        f.is_read_write()
    }

    // On Windows, we only have a `TcpStream` impl, so make a view first.
    #[cfg(windows)]
    {
        f.as_socketlike_view::<std::net::TcpStream>()
            .is_read_write()
    }
}
