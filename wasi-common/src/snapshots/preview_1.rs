use crate::{
    dir::{DirCaps, DirEntry, DirEntryExt, DirFdStat, ReaddirCursor, ReaddirEntity, TableDirExt},
    file::{
        Advice, FdFlags, FdStat, FileCaps, FileEntry, FileEntryExt, FileType, Filestat, OFlags,
        RiFlags, RoFlags, SdFlags, SiFlags, TableFileExt, WasiFile,
    },
    sched::{
        subscription::{RwEventFlags, SubscriptionResult},
        Poll, Userdata,
    },
    I32Exit, SystemTimeSpec, WasiCtx,
};
use cap_std::time::{Duration, SystemClock};
use std::convert::{TryFrom, TryInto};
use std::io::{IoSlice, IoSliceMut};
use std::ops::{Deref, DerefMut};
use wiggle::GuestPtr;

pub mod error;
use error::{Error, ErrorExt};

wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
    errors: { errno => trappable Error },
    // Note: not every function actually needs to be async, however, nearly all of them do, and
    // keeping that set the same in this macro and the wasmtime_wiggle / lucet_wiggle macros is
    // tedious, and there is no cost to having a sync function be async in this case.
    async: *,
    wasmtime: false,
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    async fn args_get<'b>(
        &mut self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.args.write_to_guest(argv_buf, argv)
    }

    async fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.args.number_elements(), self.args.cumulative_size()))
    }

    async fn environ_get<'b>(
        &mut self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.env.write_to_guest(environ_buf, environ)
    }

    async fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.env.number_elements(), self.env.cumulative_size()))
    }

    async fn clock_res_get(&mut self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        let resolution = match id {
            types::Clockid::Realtime => Ok(self.clocks.system.resolution()),
            types::Clockid::Monotonic => Ok(self.clocks.monotonic.resolution()),
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                Err(Error::badf().context("process and thread clocks are not supported"))
            }
        }?;
        Ok(resolution.as_nanos().try_into()?)
    }

    async fn clock_time_get(
        &mut self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        let precision = Duration::from_nanos(precision);
        match id {
            types::Clockid::Realtime => {
                let now = self.clocks.system.now(precision).into_std();
                let d = now
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map_err(|_| {
                        Error::trap(anyhow::Error::msg("current time before unix epoch"))
                    })?;
                Ok(d.as_nanos().try_into()?)
            }
            types::Clockid::Monotonic => {
                let now = self.clocks.monotonic.now(precision);
                let d = now.duration_since(self.clocks.creation_time);
                Ok(d.as_nanos().try_into()?)
            }
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                Err(Error::badf().context("process and thread clocks are not supported"))
            }
        }
    }

    async fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::ADVISE)?
            .advise(offset, len, advice.into())
            .await?;
        Ok(())
    }

    async fn fd_allocate(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::ALLOCATE)?
            .allocate(offset, len)
            .await?;
        Ok(())
    }

    async fn fd_close(&mut self, fd: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let fd = u32::from(fd);

        // Fail fast: If not present in table, Badf
        if !table.contains_key(fd) {
            return Err(Error::badf().context("key not in table"));
        }
        // fd_close must close either a File or a Dir handle
        if table.is::<FileEntry>(fd) {
            let _ = table.delete(fd);
        } else if table.is::<DirEntry>(fd) {
            // We cannot close preopened directories
            let dir_entry: &DirEntry = table.get(fd).unwrap();
            if dir_entry.preopen_path().is_some() {
                return Err(Error::not_supported().context("cannot close propened directory"));
            }
            drop(dir_entry);
            let _ = table.delete(fd);
        } else {
            return Err(Error::badf().context("key does not refer to file or directory"));
        }

        Ok(())
    }

    async fn fd_datasync(&mut self, fd: types::Fd) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::DATASYNC)?
            .datasync()
            .await?;
        Ok(())
    }

    async fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let file_entry: &mut FileEntry = table.get_mut(fd)?;
            let fdstat = file_entry.get_fdstat().await?;
            Ok(types::Fdstat::from(&fdstat))
        } else if table.is::<DirEntry>(fd) {
            let dir_entry: &DirEntry = table.get(fd)?;
            let dir_fdstat = dir_entry.get_dir_fdstat();
            Ok(types::Fdstat::from(&dir_fdstat))
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_fdstat_set_flags(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::FDSTAT_SET_FLAGS)?
            .set_fdflags(FdFlags::from(flags))
            .await
    }

    async fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let file_entry: &mut FileEntry = table.get_mut(fd)?;
            let file_caps = FileCaps::from(&fs_rights_base);
            file_entry.drop_caps_to(file_caps)
        } else if table.is::<DirEntry>(fd) {
            let dir_entry: &mut DirEntry = table.get_mut(fd)?;
            let dir_caps = DirCaps::from(&fs_rights_base);
            let file_caps = FileCaps::from(&fs_rights_inheriting);
            dir_entry.drop_caps_to(dir_caps, file_caps)
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let filestat = table
                .get_file_mut(fd)?
                .get_cap_mut(FileCaps::FILESTAT_GET)?
                .get_filestat()
                .await?;
            Ok(filestat.into())
        } else if table.is::<DirEntry>(fd) {
            let filestat = table
                .get_dir(fd)?
                .get_cap(DirCaps::FILESTAT_GET)?
                .get_filestat()
                .await?;
            Ok(filestat.into())
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_filestat_set_size(
        &mut self,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::FILESTAT_SET_SIZE)?
            .set_filestat_size(size)
            .await?;
        Ok(())
    }

    async fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        let fd = u32::from(fd);
        let table = self.table();
        // Validate flags
        let set_atim = fst_flags.contains(types::Fstflags::ATIM);
        let set_atim_now = fst_flags.contains(types::Fstflags::ATIM_NOW);
        let set_mtim = fst_flags.contains(types::Fstflags::MTIM);
        let set_mtim_now = fst_flags.contains(types::Fstflags::MTIM_NOW);

        let atim = systimespec(set_atim, atim, set_atim_now).map_err(|e| e.context("atim"))?;
        let mtim = systimespec(set_mtim, mtim, set_mtim_now).map_err(|e| e.context("mtim"))?;

        if table.is::<FileEntry>(fd) {
            table
                .get_file_mut(fd)
                .expect("checked that entry is file")
                .get_cap_mut(FileCaps::FILESTAT_SET_TIMES)?
                .set_times(atim, mtim)
                .await
        } else if table.is::<DirEntry>(fd) {
            table
                .get_dir(fd)
                .expect("checked that entry is dir")
                .get_cap(DirCaps::FILESTAT_SET_TIMES)?
                .set_times(".", atim, mtim, false)
                .await
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_read<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
    ) -> Result<types::Size, Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::READ)?;

        let mut guest_slices: Vec<wiggle::GuestSliceMut<u8>> =
            iovs.iter()
                .map(|iov_ptr| {
                    let iov_ptr = iov_ptr?;
                    let iov: types::Iovec = iov_ptr.read()?;
                    Ok(iov.buf.as_array(iov.buf_len).as_slice_mut()?.expect(
                        "cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                    ))
                })
                .collect::<Result<_, Error>>()?;

        let mut ioslices: Vec<IoSliceMut> = guest_slices
            .iter_mut()
            .map(|s| IoSliceMut::new(&mut *s))
            .collect();

        let bytes_read = f.read_vectored(&mut ioslices).await?;
        Ok(types::Size::try_from(bytes_read)?)
    }

    async fn fd_pread<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::READ | FileCaps::SEEK)?;

        let mut guest_slices: Vec<wiggle::GuestSliceMut<u8>> =
            iovs.iter()
                .map(|iov_ptr| {
                    let iov_ptr = iov_ptr?;
                    let iov: types::Iovec = iov_ptr.read()?;
                    Ok(iov.buf.as_array(iov.buf_len).as_slice_mut()?.expect(
                        "cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                    ))
                })
                .collect::<Result<_, Error>>()?;

        let mut ioslices: Vec<IoSliceMut> = guest_slices
            .iter_mut()
            .map(|s| IoSliceMut::new(&mut *s))
            .collect();

        let bytes_read = f.read_vectored_at(&mut ioslices, offset).await?;
        Ok(types::Size::try_from(bytes_read)?)
    }

    async fn fd_write<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
    ) -> Result<types::Size, Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::WRITE)?;

        let guest_slices: Vec<wiggle::GuestSlice<u8>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = iov_ptr.read()?;
                Ok(iov
                    .buf
                    .as_array(iov.buf_len)
                    .as_slice()?
                    .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)"))
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored(&ioslices).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn fd_pwrite<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::WRITE | FileCaps::SEEK)?;

        let guest_slices: Vec<wiggle::GuestSlice<u8>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = iov_ptr.read()?;
                Ok(iov
                    .buf
                    .as_array(iov.buf_len)
                    .as_slice()?
                    .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)"))
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored_at(&ioslices, offset).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<types::Prestat, Error> {
        let table = self.table();
        let dir_entry: &DirEntry = table.get(u32::from(fd)).map_err(|_| Error::badf())?;
        if let Some(ref preopen) = dir_entry.preopen_path() {
            let path_str = preopen.to_str().ok_or_else(|| Error::not_supported())?;
            let pr_name_len = u32::try_from(path_str.as_bytes().len())?;
            Ok(types::Prestat::Dir(types::PrestatDir { pr_name_len }))
        } else {
            Err(Error::not_supported().context("file is not a preopen"))
        }
    }

    async fn fd_prestat_dir_name<'a>(
        &mut self,
        fd: types::Fd,
        path: &GuestPtr<'a, u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: &DirEntry = table.get(u32::from(fd)).map_err(|_| Error::not_dir())?;
        if let Some(ref preopen) = dir_entry.preopen_path() {
            let path_bytes = preopen
                .to_str()
                .ok_or_else(|| Error::not_supported())?
                .as_bytes();
            let path_len = path_bytes.len();
            if path_len < path_max_len as usize {
                return Err(Error::name_too_long());
            }
            let mut p_memory = path
                .as_array(path_len as u32)
                .as_slice_mut()?
                .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
            p_memory.copy_from_slice(path_bytes);
            Ok(())
        } else {
            Err(Error::not_supported())
        }
    }
    async fn fd_renumber(&mut self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        let table = self.table();
        let from = u32::from(from);
        let to = u32::from(to);
        if !table.contains_key(from) {
            return Err(Error::badf());
        }
        if table.is_preopen(from) || table.is_preopen(to) {
            return Err(Error::not_supported().context("cannot renumber a preopen"));
        }
        let from_entry = table
            .delete(from)
            .expect("we checked that table contains from");
        table.insert_at(to, from_entry);
        Ok(())
    }

    async fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        use std::io::SeekFrom;

        let required_caps = if offset == 0 && whence == types::Whence::Cur {
            FileCaps::TELL
        } else {
            FileCaps::TELL | FileCaps::SEEK
        };

        let whence = match whence {
            types::Whence::Cur => SeekFrom::Current(offset),
            types::Whence::End => SeekFrom::End(offset),
            types::Whence::Set => SeekFrom::Start(offset as u64),
        };
        let newoffset = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(required_caps)?
            .seek(whence)
            .await?;
        Ok(newoffset)
    }

    async fn fd_sync(&mut self, fd: types::Fd) -> Result<(), Error> {
        self.table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::SYNC)?
            .sync()
            .await?;
        Ok(())
    }

    async fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize, Error> {
        // XXX should this be stream_position?
        let offset = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::TELL)?
            .seek(std::io::SeekFrom::Current(0))
            .await?;
        Ok(offset)
    }

    async fn fd_readdir<'a>(
        &mut self,
        fd: types::Fd,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        let mut bufused = 0;
        let mut buf = buf.clone();
        for entity in self
            .table()
            .get_dir(u32::from(fd))?
            .get_cap(DirCaps::READDIR)?
            .readdir(ReaddirCursor::from(cookie))
            .await?
        {
            let entity = entity?;
            let dirent_raw = dirent_bytes(types::Dirent::try_from(&entity)?);
            let dirent_len: types::Size = dirent_raw.len().try_into()?;
            let name_raw = entity.name.as_bytes();
            let name_len: types::Size = name_raw.len().try_into()?;

            // Copy as many bytes of the dirent as we can, up to the end of the buffer
            let dirent_copy_len = std::cmp::min(dirent_len, buf_len - bufused);
            buf.as_array(dirent_copy_len)
                .copy_from_slice(&dirent_raw[..dirent_copy_len as usize])?;

            // If the dirent struct wasnt compied entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF.
            if dirent_copy_len < dirent_len {
                return Ok(buf_len);
            }

            buf = buf.add(dirent_copy_len)?;
            bufused += dirent_copy_len;

            // Copy as many bytes of the name as we can, up to the end of the buffer
            let name_copy_len = std::cmp::min(name_len, buf_len - bufused);
            buf.as_array(name_copy_len)
                .copy_from_slice(&name_raw[..name_copy_len as usize])?;

            // If the dirent struct wasn't copied entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF

            if name_copy_len < name_len {
                return Ok(buf_len);
            }

            buf = buf.add(name_copy_len)?;
            bufused += name_copy_len;
        }
        Ok(bufused)
    }

    async fn path_create_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::CREATE_DIRECTORY)?
            .create_dir(path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref())
            .await
    }

    async fn path_filestat_get<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
    ) -> Result<types::Filestat, Error> {
        let filestat = self
            .table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::PATH_FILESTAT_GET)?
            .get_path_filestat(
                path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
                flags.contains(types::Lookupflags::SYMLINK_FOLLOW),
            )
            .await?;
        Ok(types::Filestat::from(filestat))
    }

    async fn path_filestat_set_times<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        let set_atim = fst_flags.contains(types::Fstflags::ATIM);
        let set_atim_now = fst_flags.contains(types::Fstflags::ATIM_NOW);
        let set_mtim = fst_flags.contains(types::Fstflags::MTIM);
        let set_mtim_now = fst_flags.contains(types::Fstflags::MTIM_NOW);

        let atim = systimespec(set_atim, atim, set_atim_now).map_err(|e| e.context("atim"))?;
        let mtim = systimespec(set_mtim, mtim, set_mtim_now).map_err(|e| e.context("mtim"))?;
        self.table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::PATH_FILESTAT_SET_TIMES)?
            .set_times(
                path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
                atim,
                mtim,
                flags.contains(types::Lookupflags::SYMLINK_FOLLOW),
            )
            .await
    }

    async fn path_link<'a>(
        &mut self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'a, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let src_dir = table
            .get_dir(u32::from(src_fd))?
            .get_cap(DirCaps::LINK_SOURCE)?;
        let target_dir = table
            .get_dir(u32::from(target_fd))?
            .get_cap(DirCaps::LINK_TARGET)?;
        let symlink_follow = src_flags.contains(types::Lookupflags::SYMLINK_FOLLOW);
        if symlink_follow {
            return Err(Error::invalid_argument()
                .context("symlink following on path_link is not supported"));
        }

        src_dir
            .hard_link(
                src_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
                target_dir.deref(),
                target_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
            )
            .await
    }

    async fn path_open<'a>(
        &mut self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        let table = self.table();
        let dirfd = u32::from(dirfd);
        if table.is::<FileEntry>(dirfd) {
            return Err(Error::not_dir());
        }
        let dir_entry = table.get_dir(dirfd)?;

        let symlink_follow = dirflags.contains(types::Lookupflags::SYMLINK_FOLLOW);

        let oflags = OFlags::from(&oflags);
        let fdflags = FdFlags::from(fdflags);
        let path = path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
        if oflags.contains(OFlags::DIRECTORY) {
            if oflags.contains(OFlags::CREATE)
                || oflags.contains(OFlags::EXCLUSIVE)
                || oflags.contains(OFlags::TRUNCATE)
            {
                return Err(Error::invalid_argument().context("directory oflags"));
            }
            let dir_caps = dir_entry.child_dir_caps(DirCaps::from(&fs_rights_base));
            let file_caps = dir_entry.child_file_caps(FileCaps::from(&fs_rights_inheriting));
            let dir = dir_entry.get_cap(DirCaps::OPEN)?;
            let child_dir = dir.open_dir(symlink_follow, path.deref()).await?;
            drop(dir);
            let fd = table.push(Box::new(DirEntry::new(
                dir_caps, file_caps, None, child_dir,
            )))?;
            Ok(types::Fd::from(fd))
        } else {
            let mut required_caps = DirCaps::OPEN;
            if oflags.contains(OFlags::CREATE) {
                required_caps = required_caps | DirCaps::CREATE_FILE;
            }

            let file_caps = dir_entry.child_file_caps(FileCaps::from(&fs_rights_base));
            let dir = dir_entry.get_cap(required_caps)?;
            let read = file_caps.contains(FileCaps::READ);
            let write = file_caps.contains(FileCaps::WRITE)
                || file_caps.contains(FileCaps::ALLOCATE)
                || file_caps.contains(FileCaps::FILESTAT_SET_SIZE);
            let file = dir
                .open_file(symlink_follow, path.deref(), oflags, read, write, fdflags)
                .await?;
            drop(dir);
            let fd = table.push(Box::new(FileEntry::new(file_caps, file)))?;
            Ok(types::Fd::from(fd))
        }
    }

    async fn path_readlink<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        let link = self
            .table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::READLINK)?
            .read_link(path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref())
            .await?
            .into_os_string()
            .into_string()
            .map_err(|_| Error::illegal_byte_sequence().context("link contents"))?;
        let link_bytes = link.as_bytes();
        let link_len = link_bytes.len();
        if link_len > buf_len as usize {
            return Err(Error::range());
        }
        let mut buf = buf
            .as_array(link_len as u32)
            .as_slice_mut()?
            .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
        buf.copy_from_slice(link_bytes);
        Ok(link_len as types::Size)
    }

    async fn path_remove_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::REMOVE_DIRECTORY)?
            .remove_dir(path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref())
            .await
    }

    async fn path_rename<'a>(
        &mut self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'a, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let src_dir = table
            .get_dir(u32::from(src_fd))?
            .get_cap(DirCaps::RENAME_SOURCE)?;
        let dest_dir = table
            .get_dir(u32::from(dest_fd))?
            .get_cap(DirCaps::RENAME_TARGET)?;
        src_dir
            .rename(
                src_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
                dest_dir.deref(),
                dest_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(),
            )
            .await
    }

    async fn path_symlink<'a>(
        &mut self,
        src_path: &GuestPtr<'a, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::SYMLINK)?
            .symlink(src_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref(), dest_path.as_str()?.expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref())
            .await
    }

    async fn path_unlink_file<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .get_cap(DirCaps::UNLINK_FILE)?
            .unlink_file(path.as_str()?
            .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)").deref())
            .await
    }

    async fn poll_oneoff<'a>(
        &mut self,
        subs: &GuestPtr<'a, types::Subscription>,
        events: &GuestPtr<'a, types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        if nsubscriptions == 0 {
            return Err(Error::invalid_argument().context("nsubscriptions must be nonzero"));
        }

        // Special-case a `poll_oneoff` which is just sleeping on a single
        // relative timer event, such as what WASI libc uses to implement sleep
        // functions. This supports all clock IDs, because POSIX says that
        // `clock_settime` doesn't effect relative sleeps.
        if nsubscriptions == 1 {
            let sub = subs.read()?;
            if let types::SubscriptionU::Clock(clocksub) = sub.u {
                if !clocksub
                    .flags
                    .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                {
                    self.sched
                        .sleep(Duration::from_nanos(clocksub.timeout))
                        .await?;
                    events.write(types::Event {
                        userdata: sub.userdata,
                        error: types::Errno::Success,
                        type_: types::Eventtype::Clock,
                        fd_readwrite: fd_readwrite_empty(),
                    })?;
                    return Ok(1);
                }
            }
        }

        let table = &mut self.table;
        // We need these refmuts to outlive Poll, which will hold the &mut dyn WasiFile inside
        let mut read_refs: Vec<(&dyn WasiFile, Userdata)> = Vec::new();
        let mut write_refs: Vec<(&dyn WasiFile, Userdata)> = Vec::new();
        let mut poll = Poll::new();

        let subs = subs.as_array(nsubscriptions);
        for sub_elem in subs.iter() {
            let sub_ptr = sub_elem?;
            let sub = sub_ptr.read()?;
            match sub.u {
                types::SubscriptionU::Clock(clocksub) => match clocksub.id {
                    types::Clockid::Monotonic => {
                        let clock = self.clocks.monotonic.deref();
                        let precision = Duration::from_nanos(clocksub.precision);
                        let duration = Duration::from_nanos(clocksub.timeout);
                        let deadline = if clocksub
                            .flags
                            .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                        {
                            self.clocks
                                .creation_time
                                .checked_add(duration)
                                .ok_or_else(|| Error::overflow().context("deadline"))?
                        } else {
                            clock
                                .now(precision)
                                .checked_add(duration)
                                .ok_or_else(|| Error::overflow().context("deadline"))?
                        };
                        poll.subscribe_monotonic_clock(
                            clock,
                            deadline,
                            precision,
                            sub.userdata.into(),
                        )
                    }
                    types::Clockid::Realtime => {
                        // POSIX specifies that functions like `nanosleep` and others use the
                        // `REALTIME` clock. But it also says that `clock_settime` has no effect
                        // on threads waiting in these functions. MONOTONIC should always have
                        // resolution at least as good as REALTIME, so we can translate a
                        // non-absolute `REALTIME` request into a `MONOTONIC` request.
                        let clock = self.clocks.monotonic.deref();
                        let precision = Duration::from_nanos(clocksub.precision);
                        let duration = Duration::from_nanos(clocksub.timeout);
                        let deadline = if clocksub
                            .flags
                            .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                        {
                            return Err(Error::not_supported());
                        } else {
                            clock
                                .now(precision)
                                .checked_add(duration)
                                .ok_or_else(|| Error::overflow().context("deadline"))?
                        };
                        poll.subscribe_monotonic_clock(
                            clock,
                            deadline,
                            precision,
                            sub.userdata.into(),
                        )
                    }
                    _ => Err(Error::invalid_argument()
                        .context("timer subscriptions only support monotonic timer"))?,
                },
                types::SubscriptionU::FdRead(readsub) => {
                    let fd = readsub.file_descriptor;
                    let file_ref = table
                        .get_file(u32::from(fd))?
                        .get_cap(FileCaps::POLL_READWRITE)?;
                    read_refs.push((file_ref, sub.userdata.into()));
                }
                types::SubscriptionU::FdWrite(writesub) => {
                    let fd = writesub.file_descriptor;
                    let file_ref = table
                        .get_file(u32::from(fd))?
                        .get_cap(FileCaps::POLL_READWRITE)?;
                    write_refs.push((file_ref, sub.userdata.into()));
                }
            }
        }

        for (f, ud) in read_refs.iter_mut() {
            poll.subscribe_read(*f, *ud);
        }
        for (f, ud) in write_refs.iter_mut() {
            poll.subscribe_write(*f, *ud);
        }

        self.sched.poll_oneoff(&mut poll).await?;

        let results = poll.results();
        let num_results = results.len();
        assert!(
            num_results <= nsubscriptions as usize,
            "results exceeds subscriptions"
        );
        let events = events.as_array(
            num_results
                .try_into()
                .expect("not greater than nsubscriptions"),
        );
        for ((result, userdata), event_elem) in results.into_iter().zip(events.iter()) {
            let event_ptr = event_elem?;
            let userdata: types::Userdata = userdata.into();
            event_ptr.write(match result {
                SubscriptionResult::Read(r) => {
                    let type_ = types::Eventtype::FdRead;
                    match r {
                        Ok((nbytes, flags)) => types::Event {
                            userdata,
                            error: types::Errno::Success,
                            type_,
                            fd_readwrite: types::EventFdReadwrite {
                                nbytes,
                                flags: types::Eventrwflags::from(&flags),
                            },
                        },
                        Err(e) => types::Event {
                            userdata,
                            error: e.downcast().map_err(Error::trap)?,
                            type_,
                            fd_readwrite: fd_readwrite_empty(),
                        },
                    }
                }
                SubscriptionResult::Write(r) => {
                    let type_ = types::Eventtype::FdWrite;
                    match r {
                        Ok((nbytes, flags)) => types::Event {
                            userdata,
                            error: types::Errno::Success,
                            type_,
                            fd_readwrite: types::EventFdReadwrite {
                                nbytes,
                                flags: types::Eventrwflags::from(&flags),
                            },
                        },
                        Err(e) => types::Event {
                            userdata,
                            error: e.downcast().map_err(Error::trap)?,
                            type_,
                            fd_readwrite: fd_readwrite_empty(),
                        },
                    }
                }
                SubscriptionResult::MonotonicClock(r) => {
                    let type_ = types::Eventtype::Clock;
                    types::Event {
                        userdata,
                        error: match r {
                            Ok(()) => types::Errno::Success,
                            Err(e) => e.downcast().map_err(Error::trap)?,
                        },
                        type_,
                        fd_readwrite: fd_readwrite_empty(),
                    }
                }
            })?;
        }

        Ok(num_results.try_into().expect("results fit into memory"))
    }

    async fn proc_exit(&mut self, status: types::Exitcode) -> anyhow::Error {
        // Check that the status is within WASI's range.
        if status < 126 {
            I32Exit(status as i32).into()
        } else {
            anyhow::Error::msg("exit with invalid exit status outside of [0..126)")
        }
    }

    async fn proc_raise(&mut self, _sig: types::Signal) -> Result<(), Error> {
        Err(Error::trap(anyhow::Error::msg("proc_raise unsupported")))
    }

    async fn sched_yield(&mut self) -> Result<(), Error> {
        self.sched.sched_yield().await
    }

    async fn random_get<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<(), Error> {
        let mut buf = buf
            .as_array(buf_len)
            .as_slice_mut()?
            .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)");
        self.random.try_fill_bytes(buf.deref_mut())?;
        Ok(())
    }

    async fn sock_accept(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        let table = self.table();
        let f = table
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::READ)?;

        let file = f.sock_accept(FdFlags::from(flags)).await?;
        let file_caps = FileCaps::READ
            | FileCaps::WRITE
            | FileCaps::FDSTAT_SET_FLAGS
            | FileCaps::POLL_READWRITE
            | FileCaps::FILESTAT_GET;

        let fd = table.push(Box::new(FileEntry::new(file_caps, file)))?;
        Ok(types::Fd::from(fd))
    }

    async fn sock_recv<'a>(
        &mut self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'a>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::READ)?;

        let mut guest_slices: Vec<wiggle::GuestSliceMut<u8>> =
            ri_data
                .iter()
                .map(|iov_ptr| {
                    let iov_ptr = iov_ptr?;
                    let iov: types::Iovec = iov_ptr.read()?;
                    Ok(iov.buf.as_array(iov.buf_len).as_slice_mut()?.expect(
                        "cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)",
                    ))
                })
                .collect::<Result<_, Error>>()?;

        let mut ioslices: Vec<IoSliceMut> = guest_slices
            .iter_mut()
            .map(|s| IoSliceMut::new(&mut *s))
            .collect();

        let (bytes_read, roflags) = f.sock_recv(&mut ioslices, RiFlags::from(ri_flags)).await?;
        Ok((types::Size::try_from(bytes_read)?, roflags.into()))
    }

    async fn sock_send<'a>(
        &mut self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'a>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::WRITE)?;

        let guest_slices: Vec<wiggle::GuestSlice<u8>> = si_data
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = iov_ptr.read()?;
                Ok(iov
                    .buf
                    .as_array(iov.buf_len)
                    .as_slice()?
                    .expect("cannot use with shared memories; see https://github.com/bytecodealliance/wasmtime/issues/5235 (TODO)"))
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.sock_send(&ioslices, SiFlags::empty()).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn sock_shutdown(&mut self, fd: types::Fd, how: types::Sdflags) -> Result<(), Error> {
        let f = self
            .table()
            .get_file_mut(u32::from(fd))?
            .get_cap_mut(FileCaps::FDSTAT_SET_FLAGS)?;

        f.sock_shutdown(SdFlags::from(how)).await
    }
}

impl From<types::Advice> for Advice {
    fn from(advice: types::Advice) -> Advice {
        match advice {
            types::Advice::Normal => Advice::Normal,
            types::Advice::Sequential => Advice::Sequential,
            types::Advice::Random => Advice::Random,
            types::Advice::Willneed => Advice::WillNeed,
            types::Advice::Dontneed => Advice::DontNeed,
            types::Advice::Noreuse => Advice::NoReuse,
        }
    }
}

impl From<&FdStat> for types::Fdstat {
    fn from(fdstat: &FdStat) -> types::Fdstat {
        types::Fdstat {
            fs_filetype: types::Filetype::from(&fdstat.filetype),
            fs_rights_base: types::Rights::from(&fdstat.caps),
            fs_rights_inheriting: types::Rights::empty(),
            fs_flags: types::Fdflags::from(fdstat.flags),
        }
    }
}

impl From<&DirFdStat> for types::Fdstat {
    fn from(dirstat: &DirFdStat) -> types::Fdstat {
        let fs_rights_base = types::Rights::from(&dirstat.dir_caps);
        let fs_rights_inheriting = types::Rights::from(&dirstat.file_caps) | fs_rights_base;
        types::Fdstat {
            fs_filetype: types::Filetype::Directory,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags: types::Fdflags::empty(),
        }
    }
}

// FileCaps can always be represented as wasi Rights
impl From<&FileCaps> for types::Rights {
    fn from(caps: &FileCaps) -> types::Rights {
        let mut rights = types::Rights::empty();
        if caps.contains(FileCaps::DATASYNC) {
            rights = rights | types::Rights::FD_DATASYNC;
        }
        if caps.contains(FileCaps::READ) {
            rights = rights | types::Rights::FD_READ;
        }
        if caps.contains(FileCaps::SEEK) {
            rights = rights | types::Rights::FD_SEEK;
        }
        if caps.contains(FileCaps::FDSTAT_SET_FLAGS) {
            rights = rights | types::Rights::FD_FDSTAT_SET_FLAGS;
        }
        if caps.contains(FileCaps::SYNC) {
            rights = rights | types::Rights::FD_SYNC;
        }
        if caps.contains(FileCaps::TELL) {
            rights = rights | types::Rights::FD_TELL;
        }
        if caps.contains(FileCaps::WRITE) {
            rights = rights | types::Rights::FD_WRITE;
        }
        if caps.contains(FileCaps::ADVISE) {
            rights = rights | types::Rights::FD_ADVISE;
        }
        if caps.contains(FileCaps::ALLOCATE) {
            rights = rights | types::Rights::FD_ALLOCATE;
        }
        if caps.contains(FileCaps::FILESTAT_GET) {
            rights = rights | types::Rights::FD_FILESTAT_GET;
        }
        if caps.contains(FileCaps::FILESTAT_SET_SIZE) {
            rights = rights | types::Rights::FD_FILESTAT_SET_SIZE;
        }
        if caps.contains(FileCaps::FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::FD_FILESTAT_SET_TIMES;
        }
        if caps.contains(FileCaps::POLL_READWRITE) {
            rights = rights | types::Rights::POLL_FD_READWRITE;
        }
        rights
    }
}

// FileCaps are a subset of wasi Rights - not all Rights have a valid representation as FileCaps
impl From<&types::Rights> for FileCaps {
    fn from(rights: &types::Rights) -> FileCaps {
        let mut caps = FileCaps::empty();
        if rights.contains(types::Rights::FD_DATASYNC) {
            caps = caps | FileCaps::DATASYNC;
        }
        if rights.contains(types::Rights::FD_READ) {
            caps = caps | FileCaps::READ;
        }
        if rights.contains(types::Rights::FD_SEEK) {
            caps = caps | FileCaps::SEEK;
        }
        if rights.contains(types::Rights::FD_FDSTAT_SET_FLAGS) {
            caps = caps | FileCaps::FDSTAT_SET_FLAGS;
        }
        if rights.contains(types::Rights::FD_SYNC) {
            caps = caps | FileCaps::SYNC;
        }
        if rights.contains(types::Rights::FD_TELL) {
            caps = caps | FileCaps::TELL;
        }
        if rights.contains(types::Rights::FD_WRITE) {
            caps = caps | FileCaps::WRITE;
        }
        if rights.contains(types::Rights::FD_ADVISE) {
            caps = caps | FileCaps::ADVISE;
        }
        if rights.contains(types::Rights::FD_ALLOCATE) {
            caps = caps | FileCaps::ALLOCATE;
        }
        if rights.contains(types::Rights::FD_FILESTAT_GET) {
            caps = caps | FileCaps::FILESTAT_GET;
        }
        if rights.contains(types::Rights::FD_FILESTAT_SET_SIZE) {
            caps = caps | FileCaps::FILESTAT_SET_SIZE;
        }
        if rights.contains(types::Rights::FD_FILESTAT_SET_TIMES) {
            caps = caps | FileCaps::FILESTAT_SET_TIMES;
        }
        if rights.contains(types::Rights::POLL_FD_READWRITE) {
            caps = caps | FileCaps::POLL_READWRITE;
        }
        caps
    }
}

// DirCaps can always be represented as wasi Rights
impl From<&DirCaps> for types::Rights {
    fn from(caps: &DirCaps) -> types::Rights {
        let mut rights = types::Rights::empty();
        if caps.contains(DirCaps::CREATE_DIRECTORY) {
            rights = rights | types::Rights::PATH_CREATE_DIRECTORY;
        }
        if caps.contains(DirCaps::CREATE_FILE) {
            rights = rights | types::Rights::PATH_CREATE_FILE;
        }
        if caps.contains(DirCaps::LINK_SOURCE) {
            rights = rights | types::Rights::PATH_LINK_SOURCE;
        }
        if caps.contains(DirCaps::LINK_TARGET) {
            rights = rights | types::Rights::PATH_LINK_TARGET;
        }
        if caps.contains(DirCaps::OPEN) {
            rights = rights | types::Rights::PATH_OPEN;
        }
        if caps.contains(DirCaps::READDIR) {
            rights = rights | types::Rights::FD_READDIR;
        }
        if caps.contains(DirCaps::READLINK) {
            rights = rights | types::Rights::PATH_READLINK;
        }
        if caps.contains(DirCaps::RENAME_SOURCE) {
            rights = rights | types::Rights::PATH_RENAME_SOURCE;
        }
        if caps.contains(DirCaps::RENAME_TARGET) {
            rights = rights | types::Rights::PATH_RENAME_TARGET;
        }
        if caps.contains(DirCaps::SYMLINK) {
            rights = rights | types::Rights::PATH_SYMLINK;
        }
        if caps.contains(DirCaps::REMOVE_DIRECTORY) {
            rights = rights | types::Rights::PATH_REMOVE_DIRECTORY;
        }
        if caps.contains(DirCaps::UNLINK_FILE) {
            rights = rights | types::Rights::PATH_UNLINK_FILE;
        }
        if caps.contains(DirCaps::PATH_FILESTAT_GET) {
            rights = rights | types::Rights::PATH_FILESTAT_GET;
        }
        if caps.contains(DirCaps::PATH_FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::PATH_FILESTAT_SET_TIMES;
        }
        if caps.contains(DirCaps::FILESTAT_GET) {
            rights = rights | types::Rights::FD_FILESTAT_GET;
        }
        if caps.contains(DirCaps::FILESTAT_SET_TIMES) {
            rights = rights | types::Rights::FD_FILESTAT_SET_TIMES;
        }
        rights
    }
}

// DirCaps are a subset of wasi Rights - not all Rights have a valid representation as DirCaps
impl From<&types::Rights> for DirCaps {
    fn from(rights: &types::Rights) -> DirCaps {
        let mut caps = DirCaps::empty();
        if rights.contains(types::Rights::PATH_CREATE_DIRECTORY) {
            caps = caps | DirCaps::CREATE_DIRECTORY;
        }
        if rights.contains(types::Rights::PATH_CREATE_FILE) {
            caps = caps | DirCaps::CREATE_FILE;
        }
        if rights.contains(types::Rights::PATH_LINK_SOURCE) {
            caps = caps | DirCaps::LINK_SOURCE;
        }
        if rights.contains(types::Rights::PATH_LINK_TARGET) {
            caps = caps | DirCaps::LINK_TARGET;
        }
        if rights.contains(types::Rights::PATH_OPEN) {
            caps = caps | DirCaps::OPEN;
        }
        if rights.contains(types::Rights::FD_READDIR) {
            caps = caps | DirCaps::READDIR;
        }
        if rights.contains(types::Rights::PATH_READLINK) {
            caps = caps | DirCaps::READLINK;
        }
        if rights.contains(types::Rights::PATH_RENAME_SOURCE) {
            caps = caps | DirCaps::RENAME_SOURCE;
        }
        if rights.contains(types::Rights::PATH_RENAME_TARGET) {
            caps = caps | DirCaps::RENAME_TARGET;
        }
        if rights.contains(types::Rights::PATH_SYMLINK) {
            caps = caps | DirCaps::SYMLINK;
        }
        if rights.contains(types::Rights::PATH_REMOVE_DIRECTORY) {
            caps = caps | DirCaps::REMOVE_DIRECTORY;
        }
        if rights.contains(types::Rights::PATH_UNLINK_FILE) {
            caps = caps | DirCaps::UNLINK_FILE;
        }
        if rights.contains(types::Rights::PATH_FILESTAT_GET) {
            caps = caps | DirCaps::PATH_FILESTAT_GET;
        }
        if rights.contains(types::Rights::PATH_FILESTAT_SET_TIMES) {
            caps = caps | DirCaps::PATH_FILESTAT_SET_TIMES;
        }
        if rights.contains(types::Rights::FD_FILESTAT_GET) {
            caps = caps | DirCaps::FILESTAT_GET;
        }
        if rights.contains(types::Rights::FD_FILESTAT_SET_TIMES) {
            caps = caps | DirCaps::FILESTAT_SET_TIMES;
        }
        caps
    }
}

impl From<&FileType> for types::Filetype {
    fn from(ft: &FileType) -> types::Filetype {
        match ft {
            FileType::Directory => types::Filetype::Directory,
            FileType::BlockDevice => types::Filetype::BlockDevice,
            FileType::CharacterDevice => types::Filetype::CharacterDevice,
            FileType::RegularFile => types::Filetype::RegularFile,
            FileType::SocketDgram => types::Filetype::SocketDgram,
            FileType::SocketStream => types::Filetype::SocketStream,
            FileType::SymbolicLink => types::Filetype::SymbolicLink,
            FileType::Unknown => types::Filetype::Unknown,
            FileType::Pipe => types::Filetype::Unknown,
        }
    }
}

macro_rules! convert_flags {
    ($from:ty, $to:ty, $($flag:ident),+) => {
        impl From<$from> for $to {
            fn from(f: $from) -> $to {
                let mut out = <$to>::empty();
                $(
                    if f.contains(<$from>::$flag) {
                        out |= <$to>::$flag;
                    }
                )+
                out
            }
        }
    }
}

macro_rules! convert_flags_bidirectional {
    ($from:ty, $to:ty, $($rest:tt)*) => {
        convert_flags!($from, $to, $($rest)*);
        convert_flags!($to, $from, $($rest)*);
    }
}

convert_flags_bidirectional!(
    FdFlags,
    types::Fdflags,
    APPEND,
    DSYNC,
    NONBLOCK,
    RSYNC,
    SYNC
);

convert_flags_bidirectional!(RiFlags, types::Riflags, RECV_PEEK, RECV_WAITALL);

convert_flags_bidirectional!(RoFlags, types::Roflags, RECV_DATA_TRUNCATED);

convert_flags_bidirectional!(SdFlags, types::Sdflags, RD, WR);

impl From<&types::Oflags> for OFlags {
    fn from(oflags: &types::Oflags) -> OFlags {
        let mut out = OFlags::empty();
        if oflags.contains(types::Oflags::CREAT) {
            out = out | OFlags::CREATE;
        }
        if oflags.contains(types::Oflags::DIRECTORY) {
            out = out | OFlags::DIRECTORY;
        }
        if oflags.contains(types::Oflags::EXCL) {
            out = out | OFlags::EXCLUSIVE;
        }
        if oflags.contains(types::Oflags::TRUNC) {
            out = out | OFlags::TRUNCATE;
        }
        out
    }
}

impl From<&OFlags> for types::Oflags {
    fn from(oflags: &OFlags) -> types::Oflags {
        let mut out = types::Oflags::empty();
        if oflags.contains(OFlags::CREATE) {
            out = out | types::Oflags::CREAT;
        }
        if oflags.contains(OFlags::DIRECTORY) {
            out = out | types::Oflags::DIRECTORY;
        }
        if oflags.contains(OFlags::EXCLUSIVE) {
            out = out | types::Oflags::EXCL;
        }
        if oflags.contains(OFlags::TRUNCATE) {
            out = out | types::Oflags::TRUNC;
        }
        out
    }
}
impl From<Filestat> for types::Filestat {
    fn from(stat: Filestat) -> types::Filestat {
        types::Filestat {
            dev: stat.device_id,
            ino: stat.inode,
            filetype: types::Filetype::from(&stat.filetype),
            nlink: stat.nlink,
            size: stat.size,
            atim: stat
                .atim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            mtim: stat
                .mtim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            ctim: stat
                .ctim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
        }
    }
}

impl TryFrom<&ReaddirEntity> for types::Dirent {
    type Error = Error;
    fn try_from(e: &ReaddirEntity) -> Result<types::Dirent, Error> {
        Ok(types::Dirent {
            d_ino: e.inode,
            d_namlen: e.name.as_bytes().len().try_into()?,
            d_type: types::Filetype::from(&e.filetype),
            d_next: e.next.into(),
        })
    }
}

fn dirent_bytes(dirent: types::Dirent) -> Vec<u8> {
    use wiggle::GuestType;
    assert_eq!(
        types::Dirent::guest_size(),
        std::mem::size_of::<types::Dirent>() as _,
        "Dirent guest repr and host repr should match"
    );
    assert_eq!(
        1,
        std::mem::size_of_val(&dirent.d_type),
        "Dirent member d_type should be endian-invariant"
    );
    let size = types::Dirent::guest_size()
        .try_into()
        .expect("Dirent is smaller than 2^32");
    let mut bytes = Vec::with_capacity(size);
    bytes.resize(size, 0);
    let ptr = bytes.as_mut_ptr().cast::<types::Dirent>();
    let guest_dirent = types::Dirent {
        d_ino: dirent.d_ino.to_le(),
        d_namlen: dirent.d_namlen.to_le(),
        d_type: dirent.d_type, // endian-invariant
        d_next: dirent.d_next.to_le(),
    };
    unsafe { ptr.write_unaligned(guest_dirent) };
    bytes
}

impl From<&RwEventFlags> for types::Eventrwflags {
    fn from(flags: &RwEventFlags) -> types::Eventrwflags {
        let mut out = types::Eventrwflags::empty();
        if flags.contains(RwEventFlags::HANGUP) {
            out = out | types::Eventrwflags::FD_READWRITE_HANGUP;
        }
        out
    }
}

fn fd_readwrite_empty() -> types::EventFdReadwrite {
    types::EventFdReadwrite {
        nbytes: 0,
        flags: types::Eventrwflags::empty(),
    }
}

fn systimespec(
    set: bool,
    ts: types::Timestamp,
    now: bool,
) -> Result<Option<SystemTimeSpec>, Error> {
    if set && now {
        Err(Error::invalid_argument())
    } else if set {
        Ok(Some(SystemTimeSpec::Absolute(
            SystemClock::UNIX_EPOCH + Duration::from_nanos(ts),
        )))
    } else if now {
        Ok(Some(SystemTimeSpec::SymbolicNow))
    } else {
        Ok(None)
    }
}
