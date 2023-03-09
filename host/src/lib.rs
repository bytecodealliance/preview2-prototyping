mod clocks;
mod console;
mod env;
mod exit;
mod filesystem;
mod io;
mod ip_name_lookup;
mod network;
mod poll;
mod random;
mod tcp;
mod udp;
pub use wasi_common::{table::Table, WasiCtx};

type HostResult<T, E> = anyhow::Result<Result<T, E>>;

pub mod wasi {
    // One day we'll be able to get wasmtime's bindgen to share code generated for these
    // two worlds. Until then, lets generate both, and ignore everything in reactor except
    // for the console (the sole item provided exclusively by the reactor world).
    pub mod command {
        wasmtime::component::bindgen!({
            path: "../wit",
            world: "command",
            tracing: true,
            async: true,
        });
    }
    pub mod reactor {
        wasmtime::component::bindgen!({
            path: "../wit",
            world: "reactor",
            tracing: true,
            async: true,
        });
    }
    pub use command::{
        environment, environment_preopens, exit, filesystem, instance_monotonic_clock,
        instance_network, instance_wall_clock, ip_name_lookup, monotonic_clock, network, poll,
        random, streams, tcp, tcp_create_socket, timezone, udp, udp_create_socket, wall_clock,
        Command,
    };
    // reactor provides a console, whereas command does not. we can get away with reusing just
    // this interface because it doesnt reuse types from any other interfaces in the reactor
    // world
    pub use reactor::console;
}

// Adds all imports available to commands and reactors to the linker. This does mean that a command
// will get away with importing a `console`, which isnt correct, but we're going to fudge it for
// until bindgen can handle sharing code between worlds better.
pub fn add_to_linker<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi::wall_clock::add_to_linker(l, f)?;
    wasi::monotonic_clock::add_to_linker(l, f)?;
    wasi::timezone::add_to_linker(l, f)?;
    wasi::instance_monotonic_clock::add_to_linker(l, f)?;
    wasi::instance_wall_clock::add_to_linker(l, f)?;
    wasi::filesystem::add_to_linker(l, f)?;
    wasi::poll::add_to_linker(l, f)?;
    wasi::streams::add_to_linker(l, f)?;
    wasi::random::add_to_linker(l, f)?;
    wasi::tcp::add_to_linker(l, f)?;
    wasi::udp::add_to_linker(l, f)?;
    wasi::ip_name_lookup::add_to_linker(l, f)?;
    wasi::instance_network::add_to_linker(l, f)?;
    wasi::network::add_to_linker(l, f)?;
    wasi::exit::add_to_linker(l, f)?;
    wasi::environment::add_to_linker(l, f)?;
    wasi::environment_preopens::add_to_linker(l, f)?;
    wasi::console::add_to_linker(l, f)?;
    Ok(())
}
