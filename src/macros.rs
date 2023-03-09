//! Minimal versions of standard-library panicking and printing macros.
//!
//! We're avoiding static initializers, so we can't have things like string
//! literals. Replace the standard assert macros with simpler implementations.

// When built as a command, the adapter exists in a world which does not have
// a logging console, but is passed stdin, stdout, and stderr as part of the
// `main` entrypoint. We write stderr OutputStream provided to `main` to a
// global before any other execution, so that error printing does not need any
// information stored in State.
#[cfg(feature = "command")]
#[allow(dead_code)]
#[doc(hidden)]
pub fn print(message: &[u8]) {
    let _ = crate::bindings::streams::write(unsafe { crate::get_stderr_stream() }, message);
}

// When built as a reactor, the adapter exists in a world which has a wasi
// logging interface at console. By convention, we use the context string
// "stderr" to provide stderr output on the console.
#[cfg(feature = "reactor")]
#[allow(dead_code)]
#[doc(hidden)]
pub fn print(message: &[u8]) {
    let stderr = byte_array::str!("stderr");
    crate::bindings::console::log(crate::bindings::console::Level::Info, &stderr, message);
}

/// A minimal `eprint` for debugging.
#[allow(unused_macros)]
macro_rules! eprint {
    ($arg:tt) => {{
        // We have to expand string literals into byte arrays to prevent them
        // from getting statically initialized.
        let message = byte_array::str!($arg);
        $crate::macros::print(&message);
    }};
}

/// A minimal `eprintln` for debugging.
#[allow(unused_macros)]
macro_rules! eprintln {
    ($arg:tt) => {{
        // We have to expand string literals into byte arrays to prevent them
        // from getting statically initialized.
        let message = byte_array::str_nl!($arg);
        $crate::macros::print(&message);
    }};
}

pub(crate) fn eprint_u32(x: u32) {
    if x == 0 {
        eprint!("0");
    } else {
        eprint_u32_impl(x)
    }

    fn eprint_u32_impl(x: u32) {
        if x != 0 {
            eprint_u32_impl(x / 10);

            let digit = [b'0' + ((x % 10) as u8)];
            crate::macros::print(&digit);
        }
    }
}

/// A minimal `unreachable`.
macro_rules! unreachable {
    () => {{
        eprint!("unreachable executed at adapter line ");
        crate::macros::eprint_u32(line!());
        eprint!("\n");
        wasm32::unreachable()
    }};

    ($arg:tt) => {{
        eprint!("unreachable executed at adapter line ");
        crate::macros::eprint_u32(line!());
        eprint!(": ");
        eprintln!($arg);
        eprint!("\n");
        wasm32::unreachable()
    }};
}

/// A minimal `assert`.
macro_rules! assert {
    ($cond:expr $(,)?) => {
        if !$cond {
            unreachable!("assertion failed")
        }
    };
}

/// A minimal `assert_eq`.
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert!($left == $right);
    };
}
