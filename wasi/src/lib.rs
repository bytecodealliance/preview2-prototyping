#[cfg(feature = "http")]
pub mod http;

pub mod snapshots {
    pub mod preview_2 {
        wit_bindgen::generate!({
            world: "reactor",
            std_feature,
        });
    }
}
