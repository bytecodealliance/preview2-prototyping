use std::{env, process};
use wasi_tests::{assert_errno, open_scratch_directory, TESTCONFIG};

unsafe fn test_path_filestat(dir_fd: wasi::Fd) {
    let fdflags = if TESTCONFIG.support_fdflags_sync() {
        wasi::FDFLAGS_APPEND | wasi::FDFLAGS_SYNC
    } else {
        wasi::FDFLAGS_APPEND
    };

    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        // Pass some flags for later retrieval
        fdflags,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    let fdstat = wasi::fd_fdstat_get(file_fd).expect("fd_fdstat_get");
    assert_eq!(
        fdstat.fs_flags & wasi::FDFLAGS_APPEND,
        wasi::FDFLAGS_APPEND,
        "file should have the APPEND fdflag used to create the file"
    );
    if TESTCONFIG.support_fdflags_sync() {
        assert_eq!(
            fdstat.fs_flags & wasi::FDFLAGS_SYNC,
            wasi::FDFLAGS_SYNC,
            "file should have the SYNC fdflag used to create the file"
        );
    }

    if !TESTCONFIG.support_fdflags_sync() {
        assert_errno!(
            wasi::path_open(
                dir_fd,
                0,
                "file",
                0,
                wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
                0,
                wasi::FDFLAGS_SYNC,
            )
            .expect_err("FDFLAGS_SYNC not supported by platform"),
            wasi::ERRNO_NOTSUP
        );
    }

    // Check file size
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "file").expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");

    // Check path_filestat_set_times
    let new_mtim = file_stat.mtim - 100;
    wasi::path_filestat_set_times(dir_fd, 0, "file", 0, new_mtim, wasi::FSTFLAGS_MTIM)
        .expect("path_filestat_set_times should succeed");

    let modified_file_stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");
    assert_eq!(modified_file_stat.mtim, new_mtim, "mtim should change");

    assert_errno!(
        wasi::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            0,
            new_mtim,
            wasi::FSTFLAGS_MTIM | wasi::FSTFLAGS_MTIM_NOW,
        )
        .expect_err("MTIM and MTIM_NOW can't both be set"),
        wasi::ERRNO_INVAL
    );

    // check if the times were untouched
    let unmodified_file_stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after ERRNO_INVAL fd_filestat_set_times");
    assert_eq!(
        unmodified_file_stat.mtim, new_mtim,
        "mtim should not change"
    );

    // Invalid arguments to set_times:
    assert_errno!(
        wasi::path_filestat_set_times(
            dir_fd,
            0,
            "file",
            0,
            0,
            wasi::FSTFLAGS_ATIM | wasi::FSTFLAGS_ATIM_NOW,
        )
        .expect_err("ATIM & ATIM_NOW can't both be set"),
        wasi::ERRNO_INVAL
    );

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
}
fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_path_filestat(dir_fd) }
}
