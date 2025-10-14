use std::ffi::{CStr, c_char};

use crate::{instance::api::with_instance, prelude::*, utils::copy_str_to_cchar_ptr};

pub extern "system" fn string_to_path(
    xr_instance: xr::Instance,
    path: *const c_char,
    xr_path: *mut xr::Path,
) -> xr::Result {
    if path.is_null() || xr_path.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (path, xr_path) = unsafe { (CStr::from_ptr(path), &mut *xr_path) };

    with_instance(xr_instance.into_raw(), |instance| {
        *xr_path = xr::Path::from_raw(instance.register_path(path)?);
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn path_to_string(
    xr_instance: xr::Instance,
    xr_path: xr::Path,
    capacity_in: u32,
    count_out: *mut u32,
    buf: *mut c_char,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    with_instance(xr_instance.into_raw(), |instance| {
        let path = instance.get_path_string(xr_path.into_raw())?;
        let path_wnull = (path.len() + 1) as u32;

        if capacity_in == 0 {
            *count_out = path_wnull;
            return Ok(());
        }

        if *count_out != path_wnull {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT.into());
        }

        if buf.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE.into());
        }

        copy_str_to_cchar_ptr::<{ xr::MAX_PATH_LENGTH }>(path, buf);
        log::debug!("path_to_string {xr_path:?}->{path}");

        Ok(())
    })
    .into_xr_result()
}
