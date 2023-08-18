#[cfg(feature = "http-client")]
pub mod http_client;

pub mod snapshots {
    pub mod preview_2 {
        wit_bindgen::generate!({
            world: "wasi:preview/reactor",
            std_feature,
            ownership: Owning,
        });
    }
}
