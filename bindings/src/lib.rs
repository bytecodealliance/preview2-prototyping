mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "reactor",
        std_feature,
        // The generated definition of command will pull in std, so we are defining it
        // manually below instead
        skip: ["command", "get-preopens", "get-environment"],
    });
}

pub use bindings::*;
