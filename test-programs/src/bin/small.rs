mod bindings {
    wit_bindgen::generate!({
        world: "little",
        path: "../wit",
    });
}

fn main() {
    bindings::stderr::print("Hello, world\n");
}
