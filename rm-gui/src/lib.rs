pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

mod app;
pub use app::Mapper;