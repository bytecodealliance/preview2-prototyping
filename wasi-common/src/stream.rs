use crate::{Error, ErrorExt};
use std::any::Any;

#[async_trait::async_trait]
pub trait WasiStream: Send + Sync {
    fn as_any(&self) -> &dyn Any;

    #[cfg(unix)]
    fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
        None
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        None
    }

    #[cfg(unix)]
    fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
        None
    }

    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        None
    }

    async fn read(&mut self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write(&mut self, _buf: &[u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn splice(&mut self, dst: &mut dyn WasiStream, nelem: u64) -> Result<u64, Error> {
        let mut nspliced = 0;

        // TODO: Optimize by splicing more than one byte at a time.
        for _ in 0..nelem {
            let mut buf = [0u8];
            let num = self.read(&mut buf).await?;
            if num == 0 {
                break;
            }
            dst.write(&buf).await?;
            nspliced += num;
        }

        Ok(nspliced)
    }

    async fn skip(&mut self, nelem: u64) -> Result<u64, Error> {
        let mut nread = 0;

        // TODO: Optimize by reading more than one byte at a time.
        for _ in 0..nelem {
            let num = self.read(&mut [0]).await?;
            if num == 0 {
                break;
            }
            nread += num;
        }

        Ok(nread)
    }

    async fn write_repeated(&mut self, byte: u8, nelem: u64) -> Result<u64, Error> {
        let mut nwritten = 0;

        // TODO: Optimize by writing more than one byte at a time.
        for _ in 0..nelem {
            let num = self.write(&[byte]).await?;
            if num == 0 {
                break;
            }
            nwritten += num;
        }

        Ok(nwritten)
    }
}

pub trait TableStreamExt {
    fn get_stream(&self, fd: u32) -> Result<&dyn WasiStream, Error>;
    fn get_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiStream>, Error>;
}
impl TableStreamExt for crate::table::Table {
    fn get_stream(&self, fd: u32) -> Result<&dyn WasiStream, Error> {
        self.get::<Box<dyn WasiStream>>(fd).map(|f| f.as_ref())
    }
    fn get_stream_mut(&mut self, fd: u32) -> Result<&mut Box<dyn WasiStream>, Error> {
        self.get_mut::<Box<dyn WasiStream>>(fd)
    }
}
