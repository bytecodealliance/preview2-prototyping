use crate::WasiCtx;

pub mod wasi {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "command",
        tracing: true,
        async: true,
    });
}

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
    wasi::stderr::add_to_linker(l, f)?;
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
    Ok(())
}
