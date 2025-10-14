pub mod error;
mod event;
mod haptics;
mod input;
mod instance;
mod loader;
mod path;
mod rendering;
mod session;
mod spaces;
mod system;
mod utils;
mod view;
mod vulkan;

pub mod prelude {
    pub use crate::error::*;
}

extern crate openxr_sys as xr;
