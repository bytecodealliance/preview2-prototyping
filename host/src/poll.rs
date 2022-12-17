use crate::{
    wasi_poll::{self, Size, StreamError, WasiFuture, WasiPoll, WasiStream},
    HostResult, WasiCtx,
};
use wasi_common::stream::TableStreamExt;

fn convert(error: wasi_common::Error) -> wasmtime::component::Error<StreamError> {
    if let Some(_errno) = error.downcast_ref() {
        wasmtime::component::Error::new(wasi_poll::StreamError {})
    } else {
        error.into().into()
    }
}

/// A pseudo-future representation.
#[derive(Copy, Clone)]
enum Future {
    /// Poll for read events.
    Read(WasiStream),
    /// Poll for write events.
    Write(WasiStream),
    // TODO: Clock,
}

#[async_trait::async_trait]
impl WasiPoll for WasiCtx {
    async fn drop_future(&mut self, future: WasiFuture) -> anyhow::Result<()> {
        self.table_mut().delete(future);
        Ok(())
    }

    async fn poll_oneoff(&mut self, futures: Vec<WasiFuture>) -> anyhow::Result<Vec<u8>> {
        use wasi_common::sched::{Poll, SubscriptionResult, Userdata};

        let mut poll = Poll::new();
        let len = futures.len();
        for (index, future) in futures.into_iter().enumerate() {
            match *self.table().get(future).map_err(convert)? {
                Future::Read(stream) => {
                    let wasi_stream: &dyn wasi_common::WasiStream =
                        self.table().get_stream(stream).map_err(convert)?;
                    poll.subscribe_read(wasi_stream, Userdata::from(index as u64));
                }
                Future::Write(stream) => {
                    let wasi_stream: &dyn wasi_common::WasiStream =
                        self.table().get_stream(stream).map_err(convert)?;
                    poll.subscribe_write(wasi_stream, Userdata::from(index as u64));
                }
            }
        }

        self.sched.poll_oneoff(&mut poll).await?;

        let mut results = vec![0u8; len];
        for (result, data) in poll.results() {
            let flag = match result {
                SubscriptionResult::Read(result) => {
                    matches!(result, Ok((_num_ready, flags)) if !flags.is_empty())
                }
                SubscriptionResult::Write(result) => {
                    matches!(result, Ok((_num_ready, flags)) if !flags.is_empty())
                }
                SubscriptionResult::MonotonicClock(result) => result.is_ok(),
            };
            results[u64::from(data) as usize] = flag as u8;
        }
        Ok(results)
    }

    async fn drop_stream(&mut self, stream: WasiStream) -> anyhow::Result<()> {
        self.table_mut().delete(stream);
        Ok(())
    }

    async fn read_stream(
        &mut self,
        stream: WasiStream,
        len: Size,
    ) -> HostResult<Vec<u8>, StreamError> {
        let s: &mut Box<dyn wasi_common::WasiStream> =
            self.table_mut().get_stream_mut(stream).map_err(convert)?;

        let mut buffer = vec![0; len.try_into().unwrap()];

        let bytes_read: u64 = s.read(&mut buffer).await.map_err(convert)?;

        buffer.shrink_to(bytes_read as usize);

        Ok(buffer)
    }

    async fn write_stream(
        &mut self,
        stream: WasiStream,
        bytes: Vec<u8>,
    ) -> HostResult<Size, StreamError> {
        let s: &mut Box<dyn wasi_common::WasiStream> =
            self.table_mut().get_stream_mut(stream).map_err(convert)?;

        let bytes_written: u64 = s.write(&bytes).await.map_err(convert)?;

        Ok(Size::try_from(bytes_written).unwrap())
    }

    async fn skip_stream(&mut self, stream: WasiStream, len: u64) -> HostResult<u64, StreamError> {
        let s: &mut Box<dyn wasi_common::WasiStream> =
            self.table_mut().get_stream_mut(stream).map_err(convert)?;

        let bytes_skipped: u64 = s.skip(len).await.map_err(convert)?;

        Ok(bytes_skipped)
    }

    async fn write_repeated_stream(
        &mut self,
        stream: WasiStream,
        byte: u8,
        len: u64,
    ) -> HostResult<u64, StreamError> {
        let s: &mut Box<dyn wasi_common::WasiStream> =
            self.table_mut().get_stream_mut(stream).map_err(convert)?;

        let bytes_written: u64 = s.write_repeated(byte, len).await.map_err(convert)?;

        Ok(bytes_written)
    }

    async fn splice_stream(
        &mut self,
        _src: WasiStream,
        _dst: WasiStream,
        _len: u64,
    ) -> HostResult<u64, StreamError> {
        // TODO: We can't get two streams at the same time because they both
        // carry the exclusive lifetime of `self`. When [`get_many_mut`] is
        // stabilized, that could allow us to add a `get_many_stream_mut` or
        // so which lets us do this.
        //
        // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
        /*
        let s: &mut Box<dyn wasi_common::WasiStream> = self
            .table_mut()
            .get_stream_mut(src)
            .map_err(convert)?;
        let d: &mut Box<dyn wasi_common::WasiStream> = self
            .table_mut()
            .get_stream_mut(dst)
            .map_err(convert)?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await.map_err(convert)?;

        Ok(bytes_spliced)
        */

        todo!()
    }

    async fn subscribe_read(&mut self, stream: WasiStream) -> anyhow::Result<WasiFuture> {
        Ok(self.table_mut().push(Box::new(Future::Read(stream)))?)
    }

    async fn subscribe_write(&mut self, stream: WasiStream) -> anyhow::Result<WasiFuture> {
        Ok(self.table_mut().push(Box::new(Future::Write(stream)))?)
    }
}
