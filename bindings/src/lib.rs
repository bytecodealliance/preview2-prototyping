mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "wasi",
        no_std,
        unchecked,
    });
}

pub use bindings::*;
