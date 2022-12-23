use once_cell::sync::OnceCell;
use std::arch::wasm32::unreachable;
use std::mem::{size_of, transmute, MaybeUninit};
use std::ptr::read_unaligned;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

wit_bindgen_guest_rust::generate!({
    path: "../wit/reverse.wit",
});

export_reverse!(Preview2);

#[no_mangle]
pub extern "C" fn _start() {
    let stdin = streams_write().create(Stream::read(wasi::FD_STDIN));
    let stdout = streams_write().create(Stream::write(wasi::FD_STDOUT));

    // Collect the command-line arguments.
    let args: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    // Collect the environment variables.
    let vars: Vec<(String, String)> = std::env::vars().collect();
    let vars: Vec<(&str, &str)> = vars.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();

    // Collect the preopens.
    let mut preopens: Vec<(wasi::Fd, String)> = Vec::new();
    let mut fd = 3;
    unsafe {
        loop {
            let prestat = match wasi::fd_prestat_get(fd) {
                Ok(prestat) => prestat,
                Err(wasi::ERRNO_BADF) => break,
                Err(_) => unreachable(),
            };

            if prestat.tag == wasi::PREOPENTYPE_DIR.raw() {
                let mut prefix = vec![0_u8; prestat.u.dir.pr_name_len + 1];
                wasi::fd_prestat_dir_name(fd, prefix.as_mut_ptr(), prestat.u.dir.pr_name_len)
                    .unwrap();
                preopens.push((fd, String::from_utf8(prefix).unwrap()));
            } else {
                break;
            }

            fd += 1;
        }
    }
    let preopens: Vec<(wasi::Fd, &str)> = preopens.iter().map(|(a, b)| (*a, b.as_str())).collect();

    // Call `command`.
    if let Err(()) = command::command(stdin, stdout, &args, &vars, &preopens) {
        unsafe {
            wasi::proc_exit(1);
        }
    }
}

struct Preview2;

impl wasi_clocks::WasiClocks for Preview2 {
    /// Read the current value of the clock.
    ///
    /// As this the clock is monotonic, calling this function repeatedly will produce
    /// a sequence of non-decreasing values.
    fn monotonic_clock_now(_clock: wasi_clocks::MonotonicClock) -> wasi_clocks::Instant {
        unsafe { wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 1).unwrap() }
    }

    /// Query the resolution of the clock.
    fn monotonic_clock_resolution(_clock: wasi_clocks::MonotonicClock) -> wasi_clocks::Instant {
        unsafe { wasi::clock_res_get(wasi::CLOCKID_MONOTONIC).unwrap() }
    }

    /// Read the current value of the clock.
    ///
    /// As this the clock is not monotonic, calling this function repeatedly will
    /// not necessarily produce a sequence of non-decreasing values.
    ///
    /// The returned timestamps represent the number of seconds since
    /// 1970-01-01T00:00:00Z, also known as [POSIX's Seconds Since the Epoch], also
    /// known as [Unix Time].
    ///
    /// The nanoseconds field of the output is always less than 1000000000.
    ///
    /// [POSIX's Seconds Since the Epoch]: https://pubs.opengroup.org/onlinepubs/9699919799/xrat/V4_xbd_chap04.html#tag_21_04_16
    /// [Unix Time]: https://en.wikipedia.org/wiki/Unix_time
    fn wall_clock_now(_clock: wasi_clocks::WallClock) -> wasi_clocks::Datetime {
        let nanos = unsafe { wasi::clock_time_get(wasi::CLOCKID_REALTIME, 1).unwrap() };
        /*
        let duration = Duration::from_nanos(nanos);
        wasi_clocks::Datetime {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        }
        */
        wasi_clocks::Datetime {
            seconds: nanos / 1_000_000_000,
            nanoseconds: (nanos % 1_000_000_000) as u32,
        }
    }

    /// Query the resolution of the clock.
    ///
    /// The nanoseconds field of the output is always less than 1000000000.
    fn wall_clock_resolution(_clock: wasi_clocks::WallClock) -> wasi_clocks::Datetime {
        let nanos = unsafe { wasi::clock_res_get(wasi::CLOCKID_REALTIME).unwrap() };
        let duration = Duration::from_nanos(nanos);
        wasi_clocks::Datetime {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        }
    }
}

impl wasi_default_clocks::WasiDefaultClocks for Preview2 {
    fn default_monotonic_clock() -> wasi_default_clocks::MonotonicClock {
        // The actual number is unused; just return something unique.
        !100
    }

    fn default_wall_clock() -> wasi_default_clocks::WallClock {
        // The actual number is unused; just return something unique.
        !101
    }
}

impl wasi_logging::WasiLogging for Preview2 {
    /// Emit a log message.
    ///
    /// A log message has a `level` describing what kind of message is being sent,
    /// a context, which is an uninterpreted string meant to help consumers group
    /// similar messages, and a string containing the message text.
    fn log(level: wasi_logging::Level, context: String, message: String) {
        <Preview2 as wasi_stderr::WasiStderr>::print(format!(
            "{:?} {}: {}",
            level, context, message
        ))
    }
}

impl wasi_stderr::WasiStderr for Preview2 {
    /// Print text to stderr.
    fn print(message: String) {
        let mut iovs = [wasi::Ciovec {
            buf: message.as_ptr(),
            buf_len: message.len(),
        }];

        while iovs[0].buf_len != 0 {
            unsafe {
                match wasi::fd_write(wasi::FD_STDERR, &iovs) {
                    Ok(n) => {
                        iovs[0].buf = iovs[0].buf.add(n);
                        iovs[0].buf_len -= n;
                    }
                    Err(wasi::ERRNO_INTR) => (),
                    Err(_) => unreachable(),
                }
            }
        }
    }

    /// Test whether stderr is known to be a terminal.
    ///
    /// This is similar to `isatty` in POSIX.
    fn is_terminal() -> bool {
        let fdstat = match unsafe { wasi::fd_fdstat_get(wasi::FD_STDERR) } {
            Ok(fdstat) => fdstat,
            Err(_) => return false,
        };
        fdstat.fs_filetype == wasi::FILETYPE_CHARACTER_DEVICE
            && (fdstat.fs_rights_base & (wasi::RIGHTS_FD_SEEK | wasi::RIGHTS_FD_TELL)) == 0
    }

    /// If stderr is a terminal and the number of columns can be determined,
    /// return it.
    fn num_columns() -> Option<u16> {
        None
    }
}

impl wasi_filesystem::WasiFilesystem for Preview2 {
    /// Provide file advisory information on a descriptor.
    ///
    /// This is similar to `posix_fadvise` in POSIX.
    fn fadvise(
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
        len: wasi_filesystem::Filesize,
        advice: wasi_filesystem::Advice,
    ) -> Result<(), wasi_filesystem::Errno> {
        let advice = match advice {
            wasi_filesystem::Advice::Normal => wasi::ADVICE_NORMAL,
            wasi_filesystem::Advice::Sequential => wasi::ADVICE_SEQUENTIAL,
            wasi_filesystem::Advice::Random => wasi::ADVICE_RANDOM,
            wasi_filesystem::Advice::WillNeed => wasi::ADVICE_WILLNEED,
            wasi_filesystem::Advice::DontNeed => wasi::ADVICE_DONTNEED,
            wasi_filesystem::Advice::NoReuse => wasi::ADVICE_NOREUSE,
        };
        unsafe { Ok(wasi::fd_advise(fd, offset, len, advice)?) }
    }

    /// Synchronize the data of a file to disk.
    ///
    /// Note: This is similar to `fdatasync` in POSIX.
    fn datasync(fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::fd_datasync(fd)?) }
    }

    /// Get flags associated with a descriptor.
    ///
    /// Note: This returns similar flags to `fcntl(fd, F_GETFL)` in POSIX.
    ///
    /// Note: This returns the value that was the `fs_flags` value returned
    /// from `fdstat_get` in earlier versions of WASI.
    fn flags(
        fd: wasi_filesystem::Descriptor,
    ) -> Result<wasi_filesystem::DescriptorFlags, wasi_filesystem::Errno> {
        let fdstat = unsafe { wasi::fd_fdstat_get(fd)? };
        let mut flags = wasi_filesystem::DescriptorFlags::empty();
        if fdstat.fs_rights_base & wasi::RIGHTS_FD_READ == wasi::RIGHTS_FD_READ {
            flags |= wasi_filesystem::DescriptorFlags::READ;
        }
        if fdstat.fs_rights_base & wasi::RIGHTS_FD_WRITE == wasi::RIGHTS_FD_WRITE {
            flags |= wasi_filesystem::DescriptorFlags::WRITE;
        }
        if fdstat.fs_flags & wasi::FDFLAGS_DSYNC == wasi::FDFLAGS_DSYNC {
            flags |= wasi_filesystem::DescriptorFlags::DSYNC;
        }
        if fdstat.fs_flags & wasi::FDFLAGS_RSYNC == wasi::FDFLAGS_RSYNC {
            flags |= wasi_filesystem::DescriptorFlags::RSYNC;
        }
        if fdstat.fs_flags & wasi::FDFLAGS_NONBLOCK == wasi::FDFLAGS_NONBLOCK {
            flags |= wasi_filesystem::DescriptorFlags::NONBLOCK;
        }
        if fdstat.fs_flags & wasi::FDFLAGS_SYNC == wasi::FDFLAGS_SYNC {
            flags |= wasi_filesystem::DescriptorFlags::SYNC;
        }
        Ok(flags)
    }

    /// Get the dynamic type of a descriptor.
    ///
    /// Note: This returns the same value as the `type` field of the `descriptor-stat`
    /// returned by `stat`, `stat-at` and similar.
    ///
    /// Note: This returns similar flags to the `st_mode & S_IFMT` value provided
    /// by `fstat` in POSIX.
    ///
    /// Note: This returns the value that was the `fs_filetype` value returned
    /// from `fdstat_get` in earlier versions of WASI.
    ///
    /// TODO: Remove the `todo-` when wit-bindgen is updated.
    fn todo_type(
        fd: wasi_filesystem::Descriptor,
    ) -> Result<wasi_filesystem::DescriptorType, wasi_filesystem::Errno> {
        let fdstat = unsafe { wasi::fd_fdstat_get(fd)? };
        Ok(fdstat.fs_filetype.into())
    }

    /// Set flags associated with a descriptor.
    ///
    /// Note: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
    ///
    /// Note: This was called `fd_fdstat_set_flags` in earlier versions of WASI.
    fn set_flags(
        fd: wasi_filesystem::Descriptor,
        flags: wasi_filesystem::DescriptorFlags,
    ) -> Result<(), wasi_filesystem::Errno> {
        let mut fs_flags = 0;
        if flags.contains(wasi_filesystem::DescriptorFlags::DSYNC) {
            fs_flags |= wasi::FDFLAGS_DSYNC;
        }
        if flags.contains(wasi_filesystem::DescriptorFlags::NONBLOCK) {
            fs_flags |= wasi::FDFLAGS_NONBLOCK;
        }
        if flags.contains(wasi_filesystem::DescriptorFlags::RSYNC) {
            fs_flags |= wasi::FDFLAGS_RSYNC;
        }
        if flags.contains(wasi_filesystem::DescriptorFlags::SYNC) {
            fs_flags |= wasi::FDFLAGS_SYNC;
        }
        unsafe { Ok(wasi::fd_fdstat_set_flags(fd, fs_flags)?) }
    }

    /// Adjust the size of an open file. If this increases the file's size, the
    /// extra bytes are filled with zeros.
    ///
    /// Note: This was called `fd_filestat_set_size` in earlier versions of WASI.
    fn set_size(
        fd: wasi_filesystem::Descriptor,
        size: wasi_filesystem::Filesize,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::fd_filestat_set_size(fd, size)?) }
    }

    /// Adjust the timestamps of an open file or directory.
    ///
    /// Note: This is similar to `futimens` in POSIX.
    ///
    /// Note: This was called `fd_filestat_set_times` in earlier versions of WASI.
    fn set_times(
        fd: wasi_filesystem::Descriptor,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> Result<(), wasi_filesystem::Errno> {
        let mut fst_flags = 0;
        let atim = match atim {
            wasi_filesystem::NewTimestamp::NoChange => 0,
            wasi_filesystem::NewTimestamp::Now => {
                fst_flags |= wasi::FSTFLAGS_ATIM | wasi::FSTFLAGS_ATIM_NOW;
                0
            }
            wasi_filesystem::NewTimestamp::Timestamp(timestamp) => {
                fst_flags |= wasi::FSTFLAGS_ATIM;
                timestamp
            }
        };
        let mtim = match mtim {
            wasi_filesystem::NewTimestamp::NoChange => 0,
            wasi_filesystem::NewTimestamp::Now => {
                fst_flags |= wasi::FSTFLAGS_MTIM | wasi::FSTFLAGS_MTIM_NOW;
                0
            }
            wasi_filesystem::NewTimestamp::Timestamp(timestamp) => {
                fst_flags |= wasi::FSTFLAGS_MTIM;
                timestamp
            }
        };

        unsafe { Ok(wasi::fd_filestat_set_times(fd, atim, mtim, fst_flags)?) }
    }

    /// Return a stream for reading from a file.
    ///
    /// Note: This allows using `read-stream`, which is similar to `read` in POSIX.
    fn read_via_stream(
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
    ) -> Result<wasi_filesystem::WasiStream, wasi_filesystem::Errno> {
        Ok(streams_write().create(Stream::read_at(fd, offset)))
    }

    /// Return a stream for writing to a file.
    ///
    /// Note: This allows using `write-stream`, which is similar to `write` in POSIX.
    fn write_via_stream(
        fd: wasi_filesystem::Descriptor,
        offset: wasi_filesystem::Filesize,
    ) -> Result<wasi_filesystem::WasiStream, wasi_filesystem::Errno> {
        Ok(streams_write().create(Stream::write_at(fd, offset)))
    }

    /// Return a stream for appending to a file.
    ///
    /// Note: This allows using `write-stream`, which is similar to `write` with
    /// `O_APPEND` in in POSIX.
    fn append_via_stream(
        fd: wasi_filesystem::Descriptor,
    ) -> Result<wasi_filesystem::WasiStream, wasi_filesystem::Errno> {
        Ok(streams_write().create(Stream::append(fd)))
    }

    /// Read from a file at a given offset.
    ///
    /// Note: This is similar to `pread` in POSIX.
    fn pread(
        fd: wasi_filesystem::Descriptor,
        len: wasi_filesystem::Size,
        offset: wasi_filesystem::Filesize,
    ) -> Result<(Vec<u8>, bool), wasi_filesystem::Errno> {
        let mut buf = vec![0_u8; len as usize];
        let iovs = [wasi::Iovec {
            buf: buf.as_mut_ptr(),
            buf_len: buf.len(),
        }];
        unsafe {
            match wasi::fd_pread(fd, &iovs, offset) {
                Ok(0) => Ok((Vec::new(), true)),
                Ok(n) => {
                    buf.truncate(n);
                    Ok((buf, false))
                }
                Err(wasi::ERRNO_INTR) => Ok((Vec::new(), false)),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// Write to a file at a given offset.
    ///
    /// Note: This is similar to `pwrite` in POSIX.
    fn pwrite(
        fd: wasi_filesystem::Descriptor,
        buf: Vec<u8>,
        offset: wasi_filesystem::Filesize,
    ) -> Result<wasi_filesystem::Size, wasi_filesystem::Errno> {
        let iovs = [wasi::Ciovec {
            buf: buf.as_ptr(),
            buf_len: buf.len(),
        }];
        unsafe {
            match wasi::fd_pwrite(fd, &iovs, offset) {
                Ok(n) => Ok(n as wasi_filesystem::Size),
                Err(wasi::ERRNO_INTR) => Ok(0),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// Read directory entries from a directory.
    ///
    /// This always returns a new stream which starts at the beginning of the
    /// directory.
    fn readdir(
        fd: wasi_filesystem::Descriptor,
    ) -> Result<wasi_filesystem::DirEntryStream, wasi_filesystem::Errno> {
        let dirfd = Self::open_at(
            fd,
            wasi_filesystem::AtFlags::empty(),
            ".".to_string(),
            wasi_filesystem::OFlags::DIRECTORY,
            wasi_filesystem::DescriptorFlags::READ,
            wasi_filesystem::Mode::READABLE,
        )?;
        Ok(streams_write().create(Stream::read_dir(dirfd)))
    }

    /// Closes a handle returned by `readdir`
    fn close_dir_entry_stream(s: wasi_filesystem::DirEntryStream) {
        streams_write().close(s)
    }

    /// Read a single directory entry from a `dir-entry-stream`.
    fn read_dir_entry(
        dir_stream: wasi_filesystem::DirEntryStream,
    ) -> Result<Option<wasi_filesystem::DirEntry>, wasi_filesystem::Errno> {
        let mut streams = streams_write();
        let stream = streams.get_mut(dir_stream);
        if let StreamKind::ReadDir(cookie, buf, buf_offset) = &mut stream.kind {
            loop {
                if buf.len() - *buf_offset >= size_of::<wasi::Dirent>() {
                    let dirent = unsafe {
                        read_unaligned::<wasi::Dirent>(buf[*buf_offset..].as_ptr().cast())
                    };
                    let dirent_len = size_of::<wasi::Dirent>() + dirent.d_namlen as usize;
                    if buf.len() - *buf_offset >= dirent_len {
                        let name = String::from_utf8(
                            buf[*buf_offset + size_of::<wasi::Dirent>()..*buf_offset + dirent_len]
                                .to_vec(),
                        )
                        .map_err(|_| wasi_filesystem::Errno::Ilseq)?;
                        let ino = if dirent.d_ino == 0 {
                            Some(dirent.d_ino)
                        } else {
                            None
                        };

                        *cookie += dirent.d_next;
                        *buf_offset += dirent_len;

                        return Ok(Some(wasi_filesystem::DirEntry {
                            ino,
                            type_: dirent.d_type.into(),
                            name,
                        }));
                    }
                }
                buf.copy_within(*buf_offset.., 0);
                *buf_offset = buf.len() - *buf_offset;
                let len = unsafe {
                    wasi::fd_readdir(
                        stream.fd,
                        buf[*buf_offset..].as_mut_ptr(),
                        buf.len() - *buf_offset,
                        *cookie,
                    )?
                };
                if len < buf.len() - *buf_offset {
                    return Ok(None);
                }
            }
        } else {
            Err(wasi_filesystem::Errno::Badf)
        }
    }

    /// Synchronize the data and metadata of a file to disk.
    ///
    /// Note: This is similar to `fsync` in POSIX.
    fn sync(fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::fd_sync(fd)?) }
    }

    /// Create a directory.
    ///
    /// Note: This is similar to `mkdirat` in POSIX.
    fn create_directory_at(
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::path_create_directory(fd, &path)?) }
    }

    /// Return the attributes of an open file or directory.
    ///
    /// Note: This is similar to `fstat` in POSIX.
    ///
    /// Note: This was called `fd_filestat_get` in earlier versions of WASI.
    fn stat(
        fd: wasi_filesystem::Descriptor,
    ) -> Result<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
        let filestat = unsafe { wasi::fd_filestat_get(fd)? };
        let type_ = filestat.filetype.into();
        Ok(wasi_filesystem::DescriptorStat {
            size: filestat.size,
            dev: filestat.dev,
            ino: filestat.ino,
            type_,
            nlink: filestat.nlink,
            atim: filestat.atim,
            mtim: filestat.mtim,
            ctim: filestat.ctim,
        })
    }

    /// Return the attributes of a file or directory.
    ///
    /// Note: This is similar to `fstatat` in POSIX.
    ///
    /// Note: This was called `fd_filestat_get` in earlier versions of WASI.
    fn stat_at(
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
    ) -> Result<wasi_filesystem::DescriptorStat, wasi_filesystem::Errno> {
        let lookupflags = lookupflags_from_at_flags(at_flags);
        let filestat = unsafe { wasi::path_filestat_get(fd, lookupflags, &path)? };
        let type_ = filestat.filetype.into();
        Ok(wasi_filesystem::DescriptorStat {
            size: filestat.size,
            dev: filestat.dev,
            ino: filestat.ino,
            type_,
            nlink: filestat.nlink,
            atim: filestat.atim,
            mtim: filestat.mtim,
            ctim: filestat.ctim,
        })
    }

    /// Adjust the timestamps of a file or directory.
    ///
    /// Note: This is similar to `utimensat` in POSIX.
    ///
    /// Note: This was called `path_filestat_set_times` in earlier versions of WASI.
    fn set_times_at(
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        atim: wasi_filesystem::NewTimestamp,
        mtim: wasi_filesystem::NewTimestamp,
    ) -> Result<(), wasi_filesystem::Errno> {
        let lookupflags = lookupflags_from_at_flags(at_flags);

        let mut fst_flags = 0;
        let atim = match atim {
            wasi_filesystem::NewTimestamp::NoChange => 0,
            wasi_filesystem::NewTimestamp::Now => {
                fst_flags |= wasi::FSTFLAGS_ATIM | wasi::FSTFLAGS_ATIM_NOW;
                0
            }
            wasi_filesystem::NewTimestamp::Timestamp(timestamp) => {
                fst_flags |= wasi::FSTFLAGS_ATIM;
                timestamp
            }
        };
        let mtim = match mtim {
            wasi_filesystem::NewTimestamp::NoChange => 0,
            wasi_filesystem::NewTimestamp::Now => {
                fst_flags |= wasi::FSTFLAGS_MTIM | wasi::FSTFLAGS_MTIM_NOW;
                0
            }
            wasi_filesystem::NewTimestamp::Timestamp(timestamp) => {
                fst_flags |= wasi::FSTFLAGS_MTIM;
                timestamp
            }
        };

        unsafe {
            Ok(wasi::path_filestat_set_times(
                fd,
                lookupflags,
                &path,
                atim,
                mtim,
                fst_flags,
            )?)
        }
    }

    /// Create a hard link.
    ///
    /// Note: This is similar to `linkat` in POSIX.
    fn link_at(
        fd: wasi_filesystem::Descriptor,
        old_at_flags: wasi_filesystem::AtFlags,
        old_path: String,
        new_descriptor: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        let lookupflags = lookupflags_from_at_flags(old_at_flags);
        unsafe {
            Ok(wasi::path_link(
                fd,
                lookupflags,
                &old_path,
                new_descriptor,
                &new_path,
            )?)
        }
    }

    /// Open a file or directory.
    ///
    /// The returned descriptor is not guaranteed to be the lowest-numbered
    /// descriptor not currently open/ it is randomized to prevent applications
    /// from depending on making assumptions about indexes, since this is
    /// error-prone in multi-threaded contexts. The returned descriptor is
    /// guaranteed to be less than 2**31.
    ///
    /// Note: This is similar to `openat` in POSIX.
    fn open_at(
        fd: wasi_filesystem::Descriptor,
        at_flags: wasi_filesystem::AtFlags,
        path: String,
        o_flags: wasi_filesystem::OFlags,
        flags: wasi_filesystem::DescriptorFlags,
        mode: wasi_filesystem::Mode,
    ) -> Result<wasi_filesystem::Descriptor, wasi_filesystem::Errno> {
        let lookupflags = lookupflags_from_at_flags(at_flags);
        let oflags = oflags_from_o_flags(o_flags);
        let (fdflags, fs_rights_base) = flags_from_descriptor_flags(flags);
        let fs_rights_inheriting = fs_rights_base;

        // preview1 doesn't support custom modes.
        if mode != wasi_filesystem::Mode::READABLE | wasi_filesystem::Mode::WRITEABLE {
            return Err(wasi_filesystem::Errno::Notsup);
        }

        unsafe {
            let fd = wasi::path_open(
                fd,
                lookupflags,
                &path,
                oflags,
                fs_rights_base,
                fs_rights_inheriting,
                fdflags,
            )?;

            Ok(fd)
        }
    }

    /// Close a file or directory handle.
    ///
    /// Until wit supports handles, use an explicit `close` function.
    ///
    /// Note: This is similar to `close` in POSIX.
    fn close(fd: wasi_filesystem::Descriptor) {
        unsafe {
            wasi::fd_close(fd).ok();
        }
    }

    /// Read the contents of a symbolic link.
    ///
    /// Note: This is similar to `readlinkat` in POSIX.
    fn readlink_at(
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> Result<String, wasi_filesystem::Errno> {
        let mut buf = vec![0_u8; 256];
        loop {
            unsafe {
                let len = wasi::path_readlink(fd, &path, buf.as_mut_ptr(), buf.len())?;
                if len < buf.len() {
                    buf.truncate(len);
                    break;
                }
                buf.resize(
                    buf.len()
                        .checked_mul(2)
                        .ok_or(wasi_filesystem::Errno::Nomem)?,
                    0,
                );
            }
        }
        String::from_utf8(buf).map_err(|_| wasi_filesystem::Errno::Ilseq)
    }

    /// Remove a directory.
    ///
    /// Return `errno::notempty` if the directory is not empty.
    ///
    /// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.
    fn remove_directory_at(
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::path_remove_directory(fd, &path)?) }
    }

    /// Rename a filesystem object.
    ///
    /// Note: This is similar to `renameat` in POSIX.
    fn rename_at(
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_descriptor: wasi_filesystem::Descriptor,
        new_path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::path_rename(fd, &old_path, new_descriptor, &new_path)?) }
    }

    /// Create a symbolic link.
    ///
    /// Note: This is similar to `symlinkat` in POSIX.
    fn symlink_at(
        fd: wasi_filesystem::Descriptor,
        old_path: String,
        new_path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::path_symlink(&old_path, fd, &new_path)?) }
    }

    /// Unlink a filesystem object that is not a directory.
    ///
    /// Return `errno::isdir` if the path refers to a directory.
    /// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
    fn unlink_file_at(
        fd: wasi_filesystem::Descriptor,
        path: String,
    ) -> Result<(), wasi_filesystem::Errno> {
        unsafe { Ok(wasi::path_unlink_file(fd, &path)?) }
    }

    /// Change the permissions of a filesystem object that is not a directory.
    ///
    /// Note that the ultimate meanings of these permissions is
    /// filesystem-specific.
    ///
    /// Note: This is similar to `fchmodat` in POSIX.
    fn change_file_permissions_at(
        _fd: wasi_filesystem::Descriptor,
        _at_flags: wasi_filesystem::AtFlags,
        _path: String,
        _mode: wasi_filesystem::Mode,
    ) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Change the permissions of a directory.
    ///
    /// Note that the ultimate meanings of these permissions is
    /// filesystem-specific.
    ///
    /// Unlike in POSIX, the `executable` flag is not reinterpreted as a "search"
    /// flag. `read` on a directory implies readability and searchability, and
    /// `execute` is not valid for directories.
    ///
    /// Note: This is similar to `fchmodat` in POSIX.
    fn change_directory_permissions_at(
        _fd: wasi_filesystem::Descriptor,
        _at_flags: wasi_filesystem::AtFlags,
        _path: String,
        _mode: wasi_filesystem::Mode,
    ) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Request a shared advisory lock for an open file.
    ///
    /// This requests a *shared* lock; more than one shared lock can be held for
    /// a file at the same time.
    ///
    /// If the open file has an exclusive lock, this function downgrades the lock
    /// to a shared lock. If it has a shared lock, this function has no effect.
    ///
    /// This requests an *advisory* lock, meaning that the file could be accessed
    /// by other programs that don't hold the lock.
    ///
    /// It is unspecified how shared locks interact with locks acquired by
    /// non-WASI programs.
    ///
    /// This function blocks until the lock can be acquired.
    ///
    /// Not all filesystems support locking; on filesystems which don't support
    /// locking, this function returns `errno::notsup`.
    ///
    /// Note: This is similar to `flock(fd, LOCK_SH)` in Unix.
    fn lock_shared(_fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Request an exclusive advisory lock for an open file.
    ///
    /// This requests an *exclusive* lock; no other locks may be held for the
    /// file while an exclusive lock is held.
    ///
    /// If the open file has a shared lock and there are no exclusive locks held
    /// for the fhile, this function upgrades the lock to an exclusive lock. If the
    /// open file already has an exclusive lock, this function has no effect.
    ///
    /// This requests an *advisory* lock, meaning that the file could be accessed
    /// by other programs that don't hold the lock.
    ///
    /// It is unspecified whether this function succeeds if the file descriptor
    /// is not opened for writing. It is unspecified how exclusive locks interact
    /// with locks acquired by non-WASI programs.
    ///
    /// This function blocks until the lock can be acquired.
    ///
    /// Not all filesystems support locking; on filesystems which don't support
    /// locking, this function returns `errno::notsup`.
    ///
    /// Note: This is similar to `flock(fd, LOCK_EX)` in Unix.
    fn lock_exclusive(_fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Request a shared advisory lock for an open file.
    ///
    /// This requests a *shared* lock; more than one shared lock can be held for
    /// a file at the same time.
    ///
    /// If the open file has an exclusive lock, this function downgrades the lock
    /// to a shared lock. If it has a shared lock, this function has no effect.
    ///
    /// This requests an *advisory* lock, meaning that the file could be accessed
    /// by other programs that don't hold the lock.
    ///
    /// It is unspecified how shared locks interact with locks acquired by
    /// non-WASI programs.
    ///
    /// This function returns `errno::wouldblock` if the lock cannot be acquired.
    ///
    /// Not all filesystems support locking; on filesystems which don't support
    /// locking, this function returns `errno::notsup`.
    ///
    /// Note: This is similar to `flock(fd, LOCK_SH | LOCK_NB)` in Unix.
    fn try_lock_shared(_fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Request an exclusive advisory lock for an open file.
    ///
    /// This requests an *exclusive* lock; no other locks may be held for the
    /// file while an exclusive lock is held.
    ///
    /// If the open file has a shared lock and there are no exclusive locks held
    /// for the fhile, this function upgrades the lock to an exclusive lock. If the
    /// open file already has an exclusive lock, this function has no effect.
    ///
    /// This requests an *advisory* lock, meaning that the file could be accessed
    /// by other programs that don't hold the lock.
    ///
    /// It is unspecified whether this function succeeds if the file descriptor
    /// is not opened for writing. It is unspecified how exclusive locks interact
    /// with locks acquired by non-WASI programs.
    ///
    /// This function returns `errno::wouldblock` if the lock cannot be acquired.
    ///
    /// Not all filesystems support locking; on filesystems which don't support
    /// locking, this function returns `errno::notsup`.
    ///
    /// Note: This is similar to `flock(fd, LOCK_EX | LOCK_NB)` in Unix.
    fn try_lock_exclusive(_fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }

    /// Release a shared or exclusive lock on an open file.
    ///
    /// Note: This is similar to `flock(fd, LOCK_UN)` in Unix.
    fn unlock(_fd: wasi_filesystem::Descriptor) -> Result<(), wasi_filesystem::Errno> {
        Err(wasi_filesystem::Errno::Notsup)
    }
}

impl From<wasi::Errno> for wasi_filesystem::Errno {
    #[inline(never)] // Disable inlining as this is bulky and relatively cold.
    fn from(errno: wasi::Errno) -> Self {
        match errno {
            wasi::ERRNO_2BIG => obscure(wasi_filesystem::Errno::Toobig),
            wasi::ERRNO_ACCES => wasi_filesystem::Errno::Access,
            wasi::ERRNO_ADDRINUSE => wasi_filesystem::Errno::Addrinuse,
            wasi::ERRNO_ADDRNOTAVAIL => wasi_filesystem::Errno::Addrnotavail,
            wasi::ERRNO_AFNOSUPPORT => wasi_filesystem::Errno::Afnosupport,
            wasi::ERRNO_AGAIN => wasi_filesystem::Errno::Again,
            wasi::ERRNO_ALREADY => wasi_filesystem::Errno::Already,
            wasi::ERRNO_BADMSG => wasi_filesystem::Errno::Badmsg,
            wasi::ERRNO_BADF => wasi_filesystem::Errno::Badf,
            wasi::ERRNO_BUSY => wasi_filesystem::Errno::Busy,
            wasi::ERRNO_CANCELED => wasi_filesystem::Errno::Canceled,
            wasi::ERRNO_CHILD => wasi_filesystem::Errno::Child,
            wasi::ERRNO_CONNABORTED => wasi_filesystem::Errno::Connaborted,
            wasi::ERRNO_CONNREFUSED => wasi_filesystem::Errno::Connrefused,
            wasi::ERRNO_CONNRESET => wasi_filesystem::Errno::Connreset,
            wasi::ERRNO_DEADLK => wasi_filesystem::Errno::Deadlk,
            wasi::ERRNO_DESTADDRREQ => wasi_filesystem::Errno::Destaddrreq,
            wasi::ERRNO_DQUOT => wasi_filesystem::Errno::Dquot,
            wasi::ERRNO_EXIST => wasi_filesystem::Errno::Exist,
            wasi::ERRNO_FAULT => wasi_filesystem::Errno::Fault,
            wasi::ERRNO_FBIG => wasi_filesystem::Errno::Fbig,
            wasi::ERRNO_HOSTUNREACH => wasi_filesystem::Errno::Hostunreach,
            wasi::ERRNO_IDRM => wasi_filesystem::Errno::Idrm,
            wasi::ERRNO_ILSEQ => wasi_filesystem::Errno::Ilseq,
            wasi::ERRNO_INPROGRESS => wasi_filesystem::Errno::Inprogress,
            wasi::ERRNO_INTR => wasi_filesystem::Errno::Intr,
            wasi::ERRNO_INVAL => wasi_filesystem::Errno::Inval,
            wasi::ERRNO_IO => wasi_filesystem::Errno::Io,
            wasi::ERRNO_ISCONN => wasi_filesystem::Errno::Isconn,
            wasi::ERRNO_ISDIR => wasi_filesystem::Errno::Isdir,
            wasi::ERRNO_LOOP => wasi_filesystem::Errno::Loop,
            wasi::ERRNO_MFILE => wasi_filesystem::Errno::Mfile,
            wasi::ERRNO_MLINK => wasi_filesystem::Errno::Mlink,
            wasi::ERRNO_MSGSIZE => wasi_filesystem::Errno::Msgsize,
            wasi::ERRNO_MULTIHOP => wasi_filesystem::Errno::Multihop,
            wasi::ERRNO_NAMETOOLONG => wasi_filesystem::Errno::Nametoolong,
            wasi::ERRNO_NETDOWN => wasi_filesystem::Errno::Netdown,
            wasi::ERRNO_NETRESET => wasi_filesystem::Errno::Netreset,
            wasi::ERRNO_NETUNREACH => wasi_filesystem::Errno::Netunreach,
            wasi::ERRNO_NFILE => wasi_filesystem::Errno::Nfile,
            wasi::ERRNO_NOBUFS => wasi_filesystem::Errno::Nobufs,
            wasi::ERRNO_NODEV => wasi_filesystem::Errno::Nodev,
            wasi::ERRNO_NOENT => wasi_filesystem::Errno::Noent,
            wasi::ERRNO_NOEXEC => wasi_filesystem::Errno::Noexec,
            wasi::ERRNO_NOLCK => wasi_filesystem::Errno::Nolck,
            wasi::ERRNO_NOLINK => wasi_filesystem::Errno::Nolink,
            wasi::ERRNO_NOMEM => wasi_filesystem::Errno::Nomem,
            wasi::ERRNO_NOMSG => wasi_filesystem::Errno::Nomsg,
            wasi::ERRNO_NOPROTOOPT => wasi_filesystem::Errno::Noprotoopt,
            wasi::ERRNO_NOSPC => wasi_filesystem::Errno::Nospc,
            wasi::ERRNO_NOSYS => wasi_filesystem::Errno::Nosys,
            wasi::ERRNO_NOTDIR => wasi_filesystem::Errno::Notdir,
            wasi::ERRNO_NOTEMPTY => wasi_filesystem::Errno::Notempty,
            wasi::ERRNO_NOTRECOVERABLE => wasi_filesystem::Errno::Notrecoverable,
            wasi::ERRNO_NOTSUP => wasi_filesystem::Errno::Notsup,
            wasi::ERRNO_NOTTY => wasi_filesystem::Errno::Notty,
            wasi::ERRNO_NXIO => wasi_filesystem::Errno::Nxio,
            wasi::ERRNO_OVERFLOW => wasi_filesystem::Errno::Overflow,
            wasi::ERRNO_OWNERDEAD => wasi_filesystem::Errno::Ownerdead,
            wasi::ERRNO_PERM => wasi_filesystem::Errno::Perm,
            wasi::ERRNO_PIPE => wasi_filesystem::Errno::Pipe,
            wasi::ERRNO_RANGE => wasi_filesystem::Errno::Range,
            wasi::ERRNO_ROFS => wasi_filesystem::Errno::Rofs,
            wasi::ERRNO_SPIPE => wasi_filesystem::Errno::Spipe,
            wasi::ERRNO_SRCH => wasi_filesystem::Errno::Srch,
            wasi::ERRNO_STALE => wasi_filesystem::Errno::Stale,
            wasi::ERRNO_TIMEDOUT => wasi_filesystem::Errno::Timedout,
            wasi::ERRNO_TXTBSY => wasi_filesystem::Errno::Txtbsy,
            wasi::ERRNO_XDEV => wasi_filesystem::Errno::Xdev,
            _ => unreachable(),
        }
    }
}

impl From<wasi::Filetype> for wasi_filesystem::DescriptorType {
    fn from(fs_filetype: wasi::Filetype) -> Self {
        match fs_filetype {
            wasi::FILETYPE_REGULAR_FILE => wasi_filesystem::DescriptorType::RegularFile,
            wasi::FILETYPE_DIRECTORY => wasi_filesystem::DescriptorType::Directory,
            wasi::FILETYPE_BLOCK_DEVICE => wasi_filesystem::DescriptorType::BlockDevice,
            wasi::FILETYPE_CHARACTER_DEVICE => wasi_filesystem::DescriptorType::CharacterDevice,
            wasi::FILETYPE_SOCKET_STREAM => wasi_filesystem::DescriptorType::Socket,
            wasi::FILETYPE_SOCKET_DGRAM => wasi_filesystem::DescriptorType::Socket,
            wasi::FILETYPE_SYMBOLIC_LINK => wasi_filesystem::DescriptorType::SymbolicLink,
            wasi::FILETYPE_UNKNOWN => wasi_filesystem::DescriptorType::Unknown,
            _ => unreachable(),
        }
    }
}

fn flags_from_descriptor_flags(
    flags: wasi_filesystem::DescriptorFlags,
) -> (wasi::Fdflags, wasi::Rights) {
    let mut fdflags = 0;
    let mut fs_rights_base = 0;
    if flags.contains(wasi_filesystem::DescriptorFlags::READ) {
        fs_rights_base |= wasi::RIGHTS_FD_READ;
    }
    if flags.contains(wasi_filesystem::DescriptorFlags::WRITE) {
        fs_rights_base |= wasi::RIGHTS_FD_WRITE;
    }
    if flags.contains(wasi_filesystem::DescriptorFlags::SYNC) {
        fdflags |= wasi::FDFLAGS_SYNC;
    }
    if flags.contains(wasi_filesystem::DescriptorFlags::DSYNC) {
        fdflags |= wasi::FDFLAGS_DSYNC;
    }
    if flags.contains(wasi_filesystem::DescriptorFlags::RSYNC) {
        fdflags |= wasi::FDFLAGS_RSYNC;
    }
    if flags.contains(wasi_filesystem::DescriptorFlags::NONBLOCK) {
        fdflags |= wasi::FDFLAGS_NONBLOCK;
    }
    (fdflags, fs_rights_base)
}

fn lookupflags_from_at_flags(flags: wasi_filesystem::AtFlags) -> wasi::Lookupflags {
    let mut lookupflags = 0;
    if flags.contains(wasi_filesystem::AtFlags::SYMLINK_FOLLOW) {
        lookupflags |= wasi::LOOKUPFLAGS_SYMLINK_FOLLOW;
    }
    lookupflags
}

fn oflags_from_o_flags(o_flags: wasi_filesystem::OFlags) -> wasi::Oflags {
    let mut oflags = 0;
    if o_flags.contains(wasi_filesystem::OFlags::CREATE) {
        oflags |= wasi::OFLAGS_CREAT;
    }
    if o_flags.contains(wasi_filesystem::OFlags::DIRECTORY) {
        oflags |= wasi::OFLAGS_DIRECTORY;
    }
    if o_flags.contains(wasi_filesystem::OFlags::EXCL) {
        oflags |= wasi::OFLAGS_EXCL;
    }
    if o_flags.contains(wasi_filesystem::OFlags::TRUNC) {
        oflags |= wasi::OFLAGS_TRUNC;
    }
    oflags
}

impl wasi_random::WasiRandom for Preview2 {
    /// Return `len` random bytes.
    ///
    /// This function must produce data from an adaquately seeded CSPRNG, so it
    /// must not block, and the returned data is always unpredictable.
    ///
    /// Deterministic environments must omit this function, rather than
    /// implementing it with deterministic data.
    fn get_random_bytes(len: u32) -> Vec<u8> {
        let mut buf = vec![0_u8; len as usize];
        unsafe {
            wasi::random_get(buf.as_mut_ptr(), buf.len()).unwrap();
        }
        buf
    }

    /// Return a random `u64` value.
    ///
    /// This function must produce data from an adaquately seeded CSPRNG, so it
    /// must not block, and the returned data is always unpredictable.
    ///
    /// Deterministic environments must omit this function, rather than
    /// implementing it with deterministic data.
    fn get_random_u64() -> u64 {
        let mut buf = 0;
        unsafe {
            let ptr: *mut u64 = &mut buf;
            wasi::random_get(ptr.cast(), size_of::<u64>()).unwrap()
        }
        buf
    }
}

impl wasi_poll::WasiPoll for Preview2 {
    /// Dispose of the specified future, after which it may no longer be used.
    fn drop_future(f: wasi_poll::WasiFuture) {
        futures_write().close(f)
    }

    /// Dispose of the specified stream, after which it may no longer be used.
    fn drop_stream(s: wasi_poll::WasiStream) {
        streams_write().close(s)
    }

    /// Read bytes from a stream.
    fn read_stream(
        stream: wasi_poll::WasiStream,
        len: wasi_poll::Size,
    ) -> Result<(Vec<u8>, bool), wasi_poll::StreamError> {
        let mut buf = vec![0_u8; len as usize];
        let mut end = false;
        let iovs = [wasi::Iovec {
            buf: buf.as_mut_ptr(),
            buf_len: buf.len(),
        }];
        let streams = streams_read();
        let io_stream = streams.get(stream);
        match io_stream.kind {
            StreamKind::Read => {
                let nread = unsafe {
                    match wasi::fd_read(io_stream.fd, &iovs) {
                        Ok(0) => {
                            end = true;
                            0
                        }
                        Ok(n) => n,
                        Err(wasi::ERRNO_INTR) => 0,
                        Err(err) => return Err(err.into()),
                    }
                };
                buf.truncate(nread);
                Ok((buf, end))
            }
            StreamKind::ReadAt(offset) => {
                let nread = unsafe {
                    match wasi::fd_pread(io_stream.fd, &iovs, offset) {
                        Ok(0) => {
                            end = true;
                            0
                        }
                        Ok(n) => n,
                        Err(wasi::ERRNO_INTR) => 0,
                        Err(err) => return Err(err.into()),
                    }
                };
                buf.truncate(nread);
                streams_write().set(stream, Stream::read_at(io_stream.fd, offset + nread as u64));
                Ok((buf, end))
            }
            _ => Err(wasi_poll::StreamError {}),
        }
    }

    /// Write bytes to a stream.
    fn write_stream(
        stream: wasi_poll::WasiStream,
        buf: Vec<u8>,
    ) -> Result<wasi_poll::Size, wasi_poll::StreamError> {
        let iovs = [wasi::Ciovec {
            buf: buf.as_ptr(),
            buf_len: buf.len(),
        }];
        let streams = streams_read();
        let io_stream = streams.get(stream);
        match io_stream.kind {
            StreamKind::Write => {
                let nwritten = match unsafe { wasi::fd_write(io_stream.fd, &iovs) } {
                    Ok(n) => n,
                    Err(wasi::ERRNO_INTR) => 0,
                    Err(err) => return Err(err.into()),
                };
                Ok(nwritten as wasi_poll::Size)
            }
            StreamKind::WriteAt(offset) => {
                let nwritten = match unsafe { wasi::fd_pwrite(io_stream.fd, &iovs, offset) } {
                    Ok(n) => n,
                    Err(wasi::ERRNO_INTR) => 0,
                    Err(err) => return Err(err.into()),
                };
                streams_write().set(
                    stream,
                    Stream::write_at(io_stream.fd, offset + nwritten as u64),
                );
                Ok(nwritten as wasi_poll::Size)
            }
            StreamKind::Append => unsafe {
                // Temporarily switch the file descriptor to append mode, do
                // the write, then switch back. This is not atomic with respect
                // to other users of the file description, but at least WASI
                // preview1 doesn't have `dup`.
                let old_fdstat = wasi::fd_fdstat_get(io_stream.fd)?;
                let old_pos = wasi::fd_tell(io_stream.fd)?;
                wasi::fd_fdstat_set_flags(
                    io_stream.fd,
                    old_fdstat.fs_flags | wasi::FDFLAGS_APPEND,
                )?;
                let result = wasi::fd_write(io_stream.fd, &iovs);
                wasi::fd_fdstat_set_flags(io_stream.fd, old_fdstat.fs_flags).unwrap();
                wasi::fd_seek(io_stream.fd, old_pos as _, wasi::WHENCE_SET).unwrap();
                let nwritten = match result {
                    Ok(n) => n,
                    Err(wasi::ERRNO_INTR) => 0,
                    Err(err) => return Err(err.into()),
                };
                Ok(nwritten as wasi_poll::Size)
            },
            _ => Err(wasi_poll::StreamError {}),
        }
    }

    /// Skip bytes from a stream.
    fn skip_stream(
        stream: wasi_poll::WasiStream,
        len: u64,
    ) -> Result<(u64, bool), wasi_poll::StreamError> {
        let len = len.try_into().unwrap_or(wasi_poll::Size::MAX);
        Self::read_stream(stream, len).map(|(buf, end)| (buf.len() as u64, end))
    }

    /// Write a byte multiple times to a stream.
    fn write_repeated_stream(
        stream: wasi_poll::WasiStream,
        byte: u8,
        len: u64,
    ) -> Result<u64, wasi_poll::StreamError> {
        let len = len.try_into().unwrap_or(usize::MAX);
        let buf = vec![byte; len];
        Self::write_stream(stream, buf).map(|nwritten| nwritten as u64)
    }

    /// Read from one stream and write to another.
    fn splice_stream(
        src: wasi_poll::WasiStream,
        dst: wasi_poll::WasiStream,
        len: u64,
    ) -> Result<(u64, bool), wasi_poll::StreamError> {
        let len = len.try_into().unwrap_or(wasi_poll::Size::MAX);
        let (buf, end) = Self::read_stream(src, len)?;
        let len = buf.len();
        let mut at = 0;
        while at != len {
            let nwritten = Self::write_stream(dst, buf[at..].to_vec())?;
            at += nwritten as usize;
        }
        Ok((len as u64, end))
    }

    /// Create a future which will resolve once either the specified stream has bytes
    /// available to read or the other end of the stream has been closed.
    fn subscribe_read(s: wasi_poll::WasiStream) -> wasi_poll::WasiFuture {
        futures_write().create(Future::Read(s))
    }

    /// Create a future which will resolve once either the specified stream is ready
    /// to accept bytes or the other end of the stream has been closed.
    fn subscribe_write(s: wasi_poll::WasiStream) -> wasi_poll::WasiFuture {
        futures_write().create(Future::Write(s))
    }

    /// Create a future which will resolve once the specified time has been reached.
    fn subscribe_monotonic_clock(
        clock: wasi_poll::MonotonicClock,
        when: wasi_poll::Instant,
    ) -> wasi_poll::WasiFuture {
        futures_write().create(Future::MonotonicClock(clock, when))
    }

    /// Poll for completion on a set of futures.
    ///
    /// The "oneoff" in the name refers to the fact that this function must do a
    /// linear scan through the entire list of subscriptions, which may be
    /// inefficient if the number is large and the same subscriptions are used
    /// many times. In the future, it may be accompanied by an API similar to
    /// Linux's `epoll` which allows sets of subscriptions to be registered and
    /// made efficiently reusable.
    ///
    /// Note that the return type would ideally be `list<bool>`, but that would
    /// be more difficult to polyfill given the current state of `wit-bindgen`.
    /// See https://github.com/bytecodealliance/preview2-prototyping/pull/11#issuecomment-1329873061
    /// for details.  For now, we use zero to mean "not ready" and non-zero to
    /// mean "ready".
    fn poll_oneoff(in_: Vec<wasi_poll::WasiFuture>) -> Vec<u8> {
        let mut subscriptions = Vec::new();
        let futures = futures_read();
        for (index, future) in in_.iter().enumerate() {
            let future = futures.get(*future);
            let sub = match future {
                Future::Read(stream) => {
                    let fd = streams_read().get(*stream).fd;
                    wasi::Subscription {
                        userdata: index as u64,
                        u: {
                            wasi::SubscriptionU {
                                tag: wasi::EVENTTYPE_FD_READ.raw(),
                                u: wasi::SubscriptionUU {
                                    fd_read: wasi::SubscriptionFdReadwrite {
                                        file_descriptor: fd,
                                    },
                                },
                            }
                        },
                    }
                }
                Future::Write(stream) => {
                    let fd = streams_read().get(*stream).fd;
                    wasi::Subscription {
                        userdata: index as u64,
                        u: {
                            wasi::SubscriptionU {
                                tag: wasi::EVENTTYPE_FD_READ.raw(),
                                u: wasi::SubscriptionUU {
                                    fd_write: wasi::SubscriptionFdReadwrite {
                                        file_descriptor: fd,
                                    },
                                },
                            }
                        },
                    }
                }
                Future::MonotonicClock(clock, when) => wasi::Subscription {
                    userdata: index as u64,
                    u: {
                        wasi::SubscriptionU {
                            tag: wasi::EVENTTYPE_CLOCK.raw(),
                            u: wasi::SubscriptionUU {
                                clock: wasi::SubscriptionClock {
                                    id: unsafe { transmute(*clock) },
                                    timeout: *when,
                                    precision: 1,
                                    flags: 0,
                                },
                            },
                        }
                    },
                },
                Future::Free(_) => unreachable(),
            };
            subscriptions.push(sub);
        }

        let mut events = vec![MaybeUninit::<wasi::Event>::uninit(); subscriptions.len()];
        let num_events = unsafe {
            wasi::poll_oneoff(
                subscriptions.as_ptr(),
                events.as_mut_ptr().cast(),
                subscriptions.len(),
            )
            .unwrap()
        };

        let mut results = vec![0_u8; num_events];
        for event in &events[..num_events] {
            let event = unsafe { event.assume_init() };
            let index = event.userdata as usize;

            results[index] = 1;

            if event.type_ == wasi::EVENTTYPE_FD_READ || event.type_ == wasi::EVENTTYPE_FD_WRITE {
                let nbytes = event.fd_readwrite.nbytes;
                let is_closed = (event.fd_readwrite.flags & wasi::EVENTRWFLAGS_FD_READWRITE_HANGUP)
                    == wasi::EVENTRWFLAGS_FD_READWRITE_HANGUP;
                match futures.get(in_[index]) {
                    Future::Read(stream) | Future::Write(stream) => {
                        let _todo = (stream, nbytes, is_closed);
                    }
                    _ => {}
                }
            }
        }
        results
    }
}

impl From<wasi::Errno> for wasi_poll::StreamError {
    fn from(_errno: wasi::Errno) -> Self {
        wasi_poll::StreamError {}
    }
}

impl wasi_tcp::WasiTcp for Preview2 {
    /// Query the specified `socket` for how many bytes are available to read.
    fn bytes_readable(s: wasi_tcp::Socket) -> Result<wasi_tcp::BytesResult, wasi_tcp::Error> {
        let _todo = s;
        todo!()
    }

    /// Query the specified `socket` for the number of bytes ready to be accepted.
    fn bytes_writable(s: wasi_tcp::Socket) -> Result<wasi_tcp::BytesResult, wasi_tcp::Error> {
        let _todo = s;
        todo!()
    }
}

impl wasi_exit::WasiExit for Preview2 {
    /// Exit the curerent instance and any linked instances.
    fn exit(status: Result<(), ()>) {
        unsafe { wasi::proc_exit(if status.is_ok() { 0 } else { 1 }) }
    }
}

enum Future {
    Read(wasi_poll::WasiStream),
    Write(wasi_poll::WasiStream),
    MonotonicClock(wasi_clocks::MonotonicClock, wasi_clocks::Instant),
    Free(Option<wasi_poll::WasiFuture>),
}

#[derive(Default)]
struct Futures {
    vec: Vec<Future>,
    free: Option<wasi_poll::WasiFuture>,
}

impl Futures {
    fn get(&self, future: wasi_poll::WasiFuture) -> &Future {
        &self.vec[future as usize]
    }

    fn create(&mut self, future: Future) -> wasi_poll::WasiFuture {
        if let Some(free) = self.free {
            let elem = &mut self.vec[free as usize];
            if let Future::Free(free) = elem {
                self.free = *free;
            } else {
                unreachable();
            }
            *elem = future;
            free
        } else {
            let index = self.vec.len();
            self.vec.push(future);
            index.try_into().unwrap()
        }
    }

    fn close(&mut self, future: wasi_poll::WasiFuture) {
        let elem = &mut self.vec[future as usize];
        *elem = Future::Free(self.free);
        self.free = Some(future);
    }
}

fn futures_read() -> RwLockReadGuard<'static, Futures> {
    futures().read().unwrap()
}

fn futures_write() -> RwLockWriteGuard<'static, Futures> {
    futures().write().unwrap()
}

fn futures() -> &'static RwLock<Futures> {
    static FUTURES: OnceCell<RwLock<Futures>> = OnceCell::new();
    FUTURES.get_or_init(Default::default)
}

struct Stream {
    fd: wasi::Fd,
    kind: StreamKind,
}

impl Stream {
    fn read(fd: wasi::Fd) -> Self {
        Self {
            fd,
            kind: StreamKind::Read,
        }
    }

    fn read_at(fd: wasi::Fd, offset: wasi::Filesize) -> Self {
        Self {
            fd,
            kind: StreamKind::ReadAt(offset),
        }
    }

    fn write(fd: wasi::Fd) -> Self {
        Self {
            fd,
            kind: StreamKind::Write,
        }
    }

    fn write_at(fd: wasi::Fd, offset: wasi::Filesize) -> Self {
        Self {
            fd,
            kind: StreamKind::WriteAt(offset),
        }
    }

    fn append(fd: wasi::Fd) -> Self {
        Self {
            fd,
            kind: StreamKind::Append,
        }
    }

    fn read_dir(fd: wasi::Fd) -> Self {
        let buf = vec![0_u8; 4096];
        // Start at the end of the buffer so that we start by doing a read.
        let buf_offset = buf.len();
        Self {
            fd,
            kind: StreamKind::ReadDir(wasi::DIRCOOKIE_START, buf, buf_offset),
        }
    }
}

enum StreamKind {
    Read,
    Write,
    ReadAt(wasi::Filesize),
    WriteAt(wasi::Filesize),
    Append,
    ReadDir(wasi::Dircookie, Vec<u8>, usize),
    Free(Option<wasi_poll::WasiStream>),
}

#[derive(Default)]
struct Streams {
    vec: Vec<Stream>,
    free: Option<wasi_poll::WasiStream>,
}

impl Streams {
    fn get(&self, stream: wasi_poll::WasiStream) -> &Stream {
        &self.vec[stream as usize]
    }

    fn get_mut(&mut self, stream: wasi_poll::WasiStream) -> &mut Stream {
        &mut self.vec[stream as usize]
    }

    fn set(&mut self, stream: wasi_poll::WasiStream, new_stream: Stream) {
        self.vec[stream as usize] = new_stream;
    }

    fn create(&mut self, stream: Stream) -> wasi_poll::WasiStream {
        if let Some(free) = self.free {
            let elem = &mut self.vec[free as usize];
            if let StreamKind::Free(free) = elem.kind {
                self.free = free;
            } else {
                unreachable();
            }
            *elem = stream;
            free
        } else {
            let index = self.vec.len();
            self.vec.push(stream);
            index.try_into().unwrap()
        }
    }

    fn close(&mut self, stream: wasi_poll::WasiStream) {
        let elem = &mut self.vec[stream as usize];
        elem.kind = StreamKind::Free(self.free);
        self.free = Some(stream);
    }
}

fn streams_read() -> RwLockReadGuard<'static, Streams> {
    streams().read().unwrap()
}

fn streams_write() -> RwLockWriteGuard<'static, Streams> {
    streams().write().unwrap()
}

fn streams() -> &'static RwLock<Streams> {
    static FILE_STREAMS: OnceCell<RwLock<Streams>> = OnceCell::new();
    FILE_STREAMS.get_or_init(Default::default)
}

// Prevent the optimizer from generating a lookup table from the match above,
// which would require a static initializer.
fn obscure<T>(x: T) -> T {
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    x
}
