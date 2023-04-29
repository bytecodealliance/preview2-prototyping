wasmtime::component::bindgen!({
    path: "../wit",
    interfaces : "
        import instance-network: sockets.instance-network
        import ip-name-lookup: sockets.ip-name-lookup
        import network: sockets.network
        import tcp-create-socket: sockets.tcp-create-socket
        import tcp: sockets.tcp
        import udp-create-socket: sockets.udp-create-socket
        import udp: sockets.udp
    ",
    tracing: true,
    async: true,
    trappable_error_type: {
        "streams"::"stream-error": Error,
    },
    with: {
        "streams": wasi_common::wasi::streams,
        "poll": wasi_common::wasi::poll,
    }
});
