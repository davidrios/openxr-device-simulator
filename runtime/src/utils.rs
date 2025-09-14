use openxr_sys as xr;
use std::ptr;

#[macro_export]
macro_rules! bind_api_fn {
    ($fn_type:ty, $fn_name:expr) => {{
        let fn_ptr: $fn_type = $fn_name;
        std::mem::transmute::<$fn_type, openxr_sys::pfn::VoidFunction>(fn_ptr)
    }};
}

pub fn copy_cstr_to_i8<const MAX: usize>(src: &[u8], dst: &mut [i8; MAX]) {
    if src.len() >= MAX {
        panic!("src is too large");
    }

    unsafe { ptr::copy_nonoverlapping(src.as_ptr() as *const i8, dst.as_mut_ptr(), src.len()) };
}

pub fn create_identity_pose() -> xr::Posef {
    xr::Posef {
        orientation: xr::Quaternionf {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        position: xr::Vector3f {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    }
}
