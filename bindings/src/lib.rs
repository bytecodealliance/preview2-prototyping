mod bindings {
    wit_bindgen_guest_rust::generate!({
        path: "../wit",
        world: "wasi",
        no_std,
        unchecked,
    });
}

pub use bindings::*;
