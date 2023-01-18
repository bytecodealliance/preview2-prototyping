use std::{env, process};
use wasi_tests::{open_scratch_directory, TESTCONFIG};

unsafe fn test_file_allocate(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ
            | wasi::RIGHTS_FD_WRITE
            | wasi::RIGHTS_FD_ALLOCATE
            | wasi::RIGHTS_FD_FILESTAT_GET,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let mut stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should be 0");

    if TESTCONFIG.support_fd_allocate() {
        // Allocate some size
        wasi::fd_allocate(file_fd, 0, 100).expect("allocating size");
        stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
        assert_eq!(stat.size, 100, "file size should be 100");

        // Allocate should not modify if less than current size
        wasi::fd_allocate(file_fd, 10, 10).expect("allocating size less than current size");
        stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
        assert_eq!(stat.size, 100, "file size should remain unchanged at 100");

        // Allocate should modify if offset+len > current_len
        wasi::fd_allocate(file_fd, 90, 20).expect("allocating size larger than current size");
        stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
        assert_eq!(stat.size, 110, "file size should increase from 100 to 110");
    }
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
    unsafe { test_file_allocate(dir_fd) }
}
