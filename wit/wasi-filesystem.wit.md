# WASI Filesystem API

WASI filesystem is a filesystem API primarily intended to let users run WASI
programs that access their files on their existing filesystems, without
significant overhead.

It is intended to be roughly portable between Unix-family platforms and
Windows, though it does not hide many of the major differences.

Paths are passed as interface-type `string`s, meaning they must consist of
a sequence of Unicode Scalar Values (USVs). Some filesystems may contain paths
which are not accessible by this API.

Some of the content and ideas here are derived from
[CloudABI](https://github.com/NuxiNL/cloudabi).

## `descriptor`
```wit
/// A "file" descriptor. In the future, this will be replaced by handle types.
type descriptor = u32
```

## `size`
```wit
/// Size of a range of bytes in memory.
type size = u32
```

## `filesize`
```wit
/// Non-negative file size or length of a region within a file.
type filesize = u64
```

## `filedelta`
```wit
/// Relative offset within a file.
type filedelta = s64
```

## `timestamp`
```wit
/// Timestamp in nanoseconds.
///
/// TODO: wasi-clocks is moving to seconds+nanoseconds.
type timestamp = u64
```

## `info`
```wit
/// Information associated with a descriptor.
///
/// Note: This was called `fdstat` in earlier versions of WASI.
record info {
    /// The type of filesystem object referenced by a descriptor.
    %type: %type,
    /// Flags associated with a descriptor.
    %flags: %flags,
}
```

## `type`
```wit
/// The type of a filesystem object referenced by a descriptor.
///
/// Note: This was called `filetype` in earlier versions of WASI.
enum %type {
    /// The type of the descriptor or file is unknown or is different from
    /// any of the other types specified.
    unknown,
    /// The descriptor refers to a block device inode.
    block-device,
    /// The descriptor refers to a character device inode.
    character-device,
    /// The descriptor refers to a directory inode.
    directory,
    /// The descriptor refers to a named pipe.
    fifo,
    /// The file refers to a symbolic link inode.
    symbolic-link,
    /// The descriptor refers to a regular file inode.
    regular-file,
    /// The descriptor refers to a socket.
    socket,
}
```

## `flags`
```wit
/// Descriptor flags.
///
/// Note: This was called `fd-flags` in earlier versions of WASI.
flags %flags {
    /// Read mode: Data can be read.
    read,
    /// Write mode: Data can be written to.
    write,
    /// Append mode: Data written to the file is always appended to the file's
    /// end.
    append,
    /// Write according to synchronized I/O data integrity completion. Only the
    /// data stored in the file is synchronized.
    dsync,
    /// Non-blocking mode.
    nonblock,
    /// Synchronized read I/O operations.
    rsync,
    /// Write according to synchronized I/O file integrity completion. In
    /// addition to synchronizing the data stored in the file, the
    /// implementation may also synchronously update the file's metadata.
    sync,
}
```

## `stat`
```wit
/// File attributes.
///
/// Note: This was called `filestat` in earlier versions of WASI.
record stat {
    /// Device ID of device containing the file.
    dev: device,
    /// File serial number.
    ino: inode,
    /// File type.
    %type: %type,
    /// Number of hard links to the file.
    nlink: linkcount,
    /// For regular files, the file size in bytes. For symbolic links, the length
    /// in bytes of the pathname contained in the symbolic link.
    size: filesize,
    /// Last data access timestamp.
    atim: timestamp,
    /// Last data modification timestamp.
    mtim: timestamp,
    /// Last file status change timestamp.
    ctim: timestamp,
}
```

## `at-flags`
```wit
/// Flags determining the method of how paths are resolved.
flags at-flags {
    /// As long as the resolved path corresponds to a symbolic link, it is expanded.
    symlink-follow,
}
```

## `o-flags`
```wit
/// Open flags used by `open-at`.
flags o-flags {
    /// Create file if it does not exist.
    create,
    /// Fail if not a directory.
    directory,
    /// Fail if file already exists.
    excl,
    /// Truncate file to size 0.
    trunc,
}
```

## `mode`
```wit
/// Permissions mode used by `open-at`, `change-permissions-at`, and similar.
flags mode {
    /// True if the resource is considered readable by the containing
    /// filesystem.
    readable,
    /// True if the resource is considered writeable by the containing
    /// filesystem.
    writeable,
    /// True if the resource is considered executable by the containing
    /// filesystem. This does not apply to directories.
    executable,
}
```

## `linkcount`
```wit
/// Number of hard links to an inode.
type linkcount = u64
```

## `device`
```wit
/// Identifier for a device containing a file system. Can be used in combination
/// with `inode` to uniquely identify a file or directory in the filesystem.
type device = u64
```

## `inode`
```wit
/// Filesystem object serial number that is unique within its file system.
type inode = u64
```

## `new-timestamp`
```wit
/// When setting a timestamp, this gives the value to set it to.
variant new-timestamp {
    /// Leave the timestamp set to its previous value.
    no-change,
    /// Set the timestamp to the current time of the system clock associated
    /// with the filesystem.
    now,
    /// Set the timestamp to the given value.
    timestamp(timestamp),
}
```

## `dirent`
```wit
/// A directory entry.
record dirent {
    /// The serial number of the file referred to by this directory entry.
    ino: inode,
    /// The length of the name of the directory entry.
    namelen: size,
    /// The type of the file referred to by this directory entry.
    %type: %type,
}
```

## `errno`
```wit
/// Error codes returned by functions.
/// Not all of these error codes are returned by the functions provided by this
/// API; some are used in higher-level library layers, and others are provided
/// merely for alignment with POSIX.
enum errno {
    /// Argument list too long. This is similar to `E2BIG` in POSIX.
    toobig,
    /// Permission denied.
    access,
    /// Address in use.
    addrinuse,
    /// Address not available.
    addrnotavail,
    /// Address family not supported.
    afnosupport,
    /// Resource unavailable, or operation would block.
    again,
    /// Connection already in progress.
    already,
    /// Bad message.
    badmsg,
    /// Device or resource busy.
    busy,
    /// Operation canceled.
    canceled,
    /// No child processes.
    child,
    /// Connection aborted.
    connaborted,
    /// Connection refused.
    connrefused,
    /// Connection reset.
    connreset,
    /// Resource deadlock would occur.
    deadlk,
    /// Destination address required.
    destaddrreq,
    /// Storage quota exceeded.
    dquot,
    /// File exists.
    exist,
    /// Bad address.
    fault,
    /// File too large.
    fbig,
    /// Host is unreachable.
    hostunreach,
    /// Identifier removed.
    idrm,
    /// Illegal byte sequence.
    ilseq,
    /// Operation in progress.
    inprogress,
    /// Interrupted function.
    intr,
    /// Invalid argument.
    inval,
    /// I/O error.
    io,
    /// Socket is connected.
    isconn,
    /// Is a directory.
    isdir,
    /// Too many levels of symbolic links.
    loop,
    /// File descriptor value too large.
    mfile,
    /// Too many links.
    mlink,
    /// Message too large.
    msgsize,
    /// Multihop attempted.
    multihop,
    /// Filename too long.
    nametoolong,
    /// Network is down.
    netdown,
    /// Connection aborted by network.
    netreset,
    /// Network unreachable.
    netunreach,
    /// Too many files open in system.
    nfile,
    /// No buffer space available.
    nobufs,
    /// No such device.
    nodev,
    /// No such file or directory.
    noent,
    /// Executable file format error.
    noexec,
    /// No locks available.
    nolck,
    /// Link has been severed.
    nolink,
    /// Not enough space.
    nomem,
    /// No message of the desired type.
    nomsg,
    /// Protocol not available.
    noprotoopt,
    /// No space left on device.
    nospc,
    /// Function not supported.
    nosys,
    /// Not a directory or a symbolic link to a directory.
    notdir,
    /// Directory not empty.
    notempty,
    /// State not recoverable.
    notrecoverable,
    /// Not supported, or operation not supported on socket.
    notsup,
    /// Inappropriate I/O control operation.
    notty,
    /// No such device or address.
    nxio,
    /// Value too large to be stored in data type.
    overflow,
    /// Previous owner died.
    ownerdead,
    /// Operation not permitted.
    perm,
    /// Broken pipe.
    pipe,
    /// Result too large.
    range,
    /// Read-only file system.
    rofs,
    /// Invalid seek.
    spipe,
    /// No such process.
    srch,
    /// Stale file handle.
    stale,
    /// Connection timed out.
    timedout,
    /// Text file busy.
    txtbsy,
    /// Cross-device link.
    xdev,
}
```

## `advice`
```wit
/// File or memory access pattern advisory information.
enum advice {
    /// The application has no advice to give on its behavior with respect to the specified data.
    normal,
    /// The application expects to access the specified data sequentially from lower offsets to higher offsets.
    sequential,
    /// The application expects to access the specified data in a random order.
    random,
    /// The application expects to access the specified data in the near future.
    will-need,
    /// The application expects that it will not access the specified data in the near future.
    dont-need,
    /// The application expects to access the specified data once and then not reuse it thereafter.
    no-reuse,
}
```

## `seek-from`
```wit
/// The position relative to which to set the offset of the descriptor.
variant seek-from {
    /// Seek relative to start-of-file.
    set(filesize),
    /// Seek relative to current position.
    cur(filedelta),
    /// Seek relative to end-of-file.
    end(filesize),
}
```

## `fadvise`
```wit
/// Provide file advisory information on a descriptor.
///
/// This is similar to `posix_fadvise` in POSIX.
fadvise: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The offset within the file to which the advisory applies.
    offset: filesize,
    /// The length of the region to which the advisory applies.
    len: filesize,
    /// The advice.
    advice: advice
) -> result<_, errno>
```

## `fallocate`
```wit
/// Force the allocation of space in a file.
///
/// Note: This is similar to `posix_fallocate` in POSIX.
fallocate: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The offset at which to start the allocation.
    offset: filesize,
    /// The length of the area that is allocated.
    len: filesize
) -> result<_, errno>
```

## `datasync`
```wit
/// Synchronize the data of a file to disk.
///
/// Note: This is similar to `fdatasync` in POSIX.
datasync: func(
    /// The resource to operate on.
    fd: descriptor,
) -> result<_, errno>
```

## `fd-info`
```wit
/// Get information associated with a descriptor.
///
/// Note: This returns similar flags to `fcntl(fd, F_GETFL)` in POSIX, as well
/// as additional fields.
///
/// Note: This was called `fdstat_get` in earlier versions of WASI.
fd-info: func(
    /// The resource to operate on.
    fd: descriptor,
) -> result<info, errno>
```

## `set-size`
```wit
/// Adjust the size of an open file. If this increases the file's size, the
/// extra bytes are filled with zeros.
///
/// Note: This was called `fd_filestat_set_size` in earlier versions of WASI.
set-size: func(
    /// The resource to operate on.
    fd: descriptor,
    size: filesize
) -> result<_, errno>
```

## `set-times`
```wit
/// Adjust the timestamps of an open file or directory.
///
/// Note: This is similar to `futimens` in POSIX.
///
/// Note: This was called `fd_filestat_set_times` in earlier versions of WASI.
set-times: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The desired values of the data access timestamp.
    atim: new-timestamp,
    /// The desired values of the data modification timestamp.
    mtim: new-timestamp,
) -> result<_, errno>
```

## `pread`
```wit
/// Read from a descriptor, without using and updating the descriptor's offset.
///
/// Note: This is similar to `pread` in POSIX.
pread: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The maximum number of bytes to read.
    len: size,
    /// The offset within the file at which to read.
    offset: filesize,
) -> result<list<u8>, errno>
```

## `pwrite`
```wit
/// Write to a descriptor, without using and updating the descriptor's offset.
///
/// Note: This is similar to `pwrite` in POSIX.
pwrite: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Data to write
    buf: list<u8>,
    /// The offset within the file at which to write.
    offset: filesize,
) -> result<size, errno>
```

## `readdir`
```wit
/// Read directory entries from a directory.
///
/// TODO this shouldnt be a binary interface. Instead, define the struct
/// and whatever of its members are required here, and then return a list
/// of those structs. Delete the rewind argument.
///
/// When successful, the contents of the output buffer consist of a sequence of
/// directory entries. Each directory entry consists of a `dirent` object,
/// followed by `dirent::d_namlen` bytes holding the name of the directory
/// entry.
///
/// This function fills the output buffer as much as possible, potentially
/// truncating the last directory entry. This allows the caller to grow its
/// read buffer size in case it's too small to fit a single large directory
/// entry, or skip the oversized directory entry.
readdir: func(
    /// The resource to operate on.
    fd: descriptor,
    /// If true, rewind the current position to the beginning before reading.
    rewind: bool
) -> result<list<u8>, errno>
```

## `seek`
```wit
/// Move the offset of a descriptor.
///
/// The meaning of `seek` on a directory is unspecified.
///
/// Returns new offset of the descriptor, relative to the start of the file.
///
/// Note: This is similar to `lseek` in POSIX.
seek: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The method to compute the new offset.
    %from: seek-from,
) -> result<filesize, errno>
```

## `sync`
```wit
/// Synchronize the data and metadata of a file to disk.
///
/// Note: This is similar to `fsync` in POSIX.
sync: func(
    /// The resource to operate on.
    fd: descriptor,
) -> result<_, errno>
```

## `tell`
```wit
/// Return the current offset of a descriptor.
///
/// Returns the current offset of the descriptor, relative to the start of the file.
///
/// Note: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
tell: func(
    /// The resource to operate on.
    fd: descriptor,
) -> result<filesize, errno>
```

## `create-directory-at`
```wit
/// Create a directory.
///
/// Note: This is similar to `mkdirat` in POSIX.
create-directory-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The relative path at which to create the directory.
    path: string,
) -> result<_, errno>
```

## `stat-at`
```wit
/// Return the attributes of a file or directory.
///
/// Note: This is similar to `fstatat` in POSIX.
///
/// Note: This was called `fd_filestat_get` in earlier versions of WASI.
stat-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    at-flags: at-flags,
    /// The relative path of the file or directory to inspect.
    path: string,
) -> result<stat, errno>
```

## `set-times-at`
```wit
/// Adjust the timestamps of a file or directory.
///
/// Note: This is similar to `utimensat` in POSIX.
///
/// Note: This was called `path_filestat_set_times` in earlier versions of WASI.
set-times-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    at-flags: at-flags,
    /// The relative path of the file or directory to operate on.
    path: string,
    /// The desired values of the data access timestamp.
    atim: new-timestamp,
    /// The desired values of the data modification timestamp.
    mtim: new-timestamp,
) -> result<_, errno>
```

## `link-at`
```wit
/// Create a hard link.
///
/// Note: This is similar to `linkat` in POSIX.
link-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    old-at-flags: at-flags,
    /// The relative source path from which to link.
    old-path: string,
    /// The base directory for `new-path`.
    new-descriptor: descriptor,
    /// The relative destination path at which to create the hard link.
    new-path: string,
) -> result<_, errno>
```

## `open-at`
```wit
/// Open a file or directory.
///
/// The returned descriptor is not guaranteed to be the lowest-numbered
/// descriptor not currently open/ it is randomized to prevent applications
/// from depending on making assumptions about indexes, since this is
/// error-prone in multi-threaded contexts. The returned descriptor is
/// guaranteed to be less than 2**31.
///
/// Note: This is similar to `openat` in POSIX.
open-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    at-flags: at-flags,
    /// The relative path of the object to open.
    path: string,
    /// The method by which to open the file.
    o-flags: o-flags,
    /// Flags to use for the resulting descriptor.
    %flags: %flags,
    /// Permissions to use when creating a new file.
    mode: mode
) -> result<descriptor, errno>
```

## `readlink-at`
```wit
/// Read the contents of a symbolic link.
///
/// Note: This is similar to `readlinkat` in POSIX.
readlink-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The relative path of the symbolic link from which to read.
    path: string,
) -> result<string, errno>
```

## `remove-directory-at`
```wit
/// Remove a directory.
///
/// Return `errno::notempty` if the directory is not empty.
///
/// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.
remove-directory-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The relative path to a directory to remove.
    path: string,
) -> result<_, errno>
```

## `rename-at`
```wit
/// Rename a filesystem object.
///
/// Note: This is similar to `renameat` in POSIX.
rename-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The relative source path of the file or directory to rename.
    old-path: string,
    /// The base directory for `new-path`.
    new-descriptor: descriptor,
    /// The relative destination path to which to rename the file or directory.
    new-path: string,
) -> result<_, errno>
```

## `symlink-at`
```wit
/// Create a symbolic link.
///
/// Note: This is similar to `symlinkat` in POSIX.
symlink-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The contents of the symbolic link.
    old-path: string,
    /// The relative destination path at which to create the symbolic link.
    new-path: string,
) -> result<_, errno>
```

## `unlink-file-at`
```wit
/// Unlink a filesystem object that is not a directory.
///
/// Return `errno::isdir` if the path refers to a directory.
/// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
unlink-file-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// The relative path to a file to unlink.
    path: string,
) -> result<_, errno>
```

## `change-file-permissions-at`
```wit
/// Change the permissions of a filesystem object that is not a directory.
///
/// Note that the ultimate meanings of these permissions is
/// filesystem-specific.
///
/// Note: This is similar to `fchmodat` in POSIX.
change-file-permissions-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    at-flags: at-flags,
    /// The relative path to operate on.
    path: string,
    /// The new permissions for the filesystem object.
    mode: mode,
) -> result<_, errno>
```

## `change-dir-permissions-at`
```wit
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
change-directory-permissions-at: func(
    /// The resource to operate on.
    fd: descriptor,
    /// Flags determining the method of how the path is resolved.
    at-flags: at-flags,
    /// The relative path to operate on.
    path: string,
    /// The new permissions for the directory.
    mode: mode,
) -> result<_, errno>
```
