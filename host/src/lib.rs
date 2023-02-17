mod clocks;
mod env;
mod exit;
mod filesystem;
mod io;
mod ip_name_lookup;
mod logging;
mod network;
mod poll;
mod random;
mod stderr;
mod tcp;
mod udp;
pub use wasi_common::{table::Table, WasiCtx};

type HostResult<T, E> = anyhow::Result<Result<T, E>>;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "wasi-command",
    tracing: true,
    async: true,
});

pub fn add_to_linker<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi_wall_clock::add_to_linker(l, f)?;
    wasi_monotonic_clock::add_to_linker(l, f)?;
    wasi_default_clocks::add_to_linker(l, f)?;
    wasi_filesystem::add_to_linker(l, f)?;
    wasi_logging::add_to_linker(l, f)?;
    wasi_stderr::add_to_linker(l, f)?;
    wasi_poll::add_to_linker(l, f)?;
    wasi_io::add_to_linker(l, f)?;
    wasi_random::add_to_linker(l, f)?;
    wasi_tcp::add_to_linker(l, f)?;
    wasi_udp::add_to_linker(l, f)?;
    wasi_ip_name_lookup::add_to_linker(l, f)?;
    wasi_default_network::add_to_linker(l, f)?;
    wasi_network::add_to_linker(l, f)?;
    wasi_exit::add_to_linker(l, f)?;
    wasi_environment::add_to_linker(l, f)?;
    Ok(())
}
