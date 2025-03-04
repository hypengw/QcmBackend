pub mod model {
    include!(concat!(env!("OUT_DIR"), "/qcm.model.rs"));
}
mod msg {
    include!(concat!(env!("OUT_DIR"), "/qcm.message.rs"));
}

pub use msg::*;