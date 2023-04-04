use crate::WasiCtx;

pub mod wasi {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "command",
        tracing: true,
        async: true,
        trappable_error_type: {
            "filesystem"::"error-code": Error,
            "streams"::"stream-error": Error,
        },
        only_interfaces: true,
    });

    wasmtime::component::bindgen!({
        path: "../wit",
        world: "command",
        tracing: true,
        async: true,
        trappable_error_type: {
            "filesystem"::"error-code": Error,
            "streams"::"stream-error": Error,
        },
        with: {
            "filesystem": filesystem,
            "instance_monotonic_clock": instance_monotonic_clock,
            "instance_network": instance_network,
            "instance_wall_clock": instance_wall_clock,
            "ip_name_lookup": ip_name_lookup,
            "monotonic_clock": monotonic_clock,
            "network": network,
            "poll": poll,
            "streams": streams,
            "tcp": tcp,
            "tcp_create_socket": tcp_create_socket,
            "timezone": timezone,
            "udp": udp,
            "udp_create_socket": udp_create_socket,
            "wall_clock": wall_clock,
            "random": random,
            "environment": environment,
            "exit": exit,
            "preopens": preopens,
        },
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
    wasi::poll::add_to_linker(l, f)?;
    wasi::streams::add_to_linker(l, f)?;
    wasi::random::add_to_linker(l, f)?;
    wasi::tcp::add_to_linker(l, f)?;
    wasi::tcp_create_socket::add_to_linker(l, f)?;
    wasi::udp::add_to_linker(l, f)?;
    wasi::udp_create_socket::add_to_linker(l, f)?;
    wasi::ip_name_lookup::add_to_linker(l, f)?;
    wasi::instance_network::add_to_linker(l, f)?;
    wasi::network::add_to_linker(l, f)?;
    wasi::exit::add_to_linker(l, f)?;
    wasi::environment::add_to_linker(l, f)?;
    wasi::preopens::add_to_linker(l, f)?;
    Ok(())
}
