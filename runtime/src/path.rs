use std::ffi::{CStr, c_char};

use openxr_sys as xr;

use crate::{error::to_xr_result, utils::copy_str_to_cchar_ptr, with_instance};

pub extern "system" fn string_to_path(
    xr_instance: xr::Instance,
    path: *const c_char,
    xr_path: *mut xr::Path,
) -> xr::Result {
    if path.is_null() || xr_path.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (path, xr_path) = unsafe { (CStr::from_ptr(path), &mut *xr_path) };

    to_xr_result(with_instance!(xr_instance, |instance| {
        *xr_path = xr::Path::from_raw(match instance.register_path(path) {
            Ok(id) => id,
            Err(err) => return err.into(),
        });
        Ok(())
    }))
}

pub extern "system" fn path_to_string(
    xr_instance: xr::Instance,
    xr_path: xr::Path,
    capacity_in: u32,
    count_out: *mut u32,
    buf: *mut c_char,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_instance!(xr_instance, |instance| {
        let path = match instance.get_path_string(xr_path) {
            Ok(path) => path,
            Err(err) => return err.into(),
        };
        let path_wnull = (path.len() + 1) as u32;

        if capacity_in == 0 {
            *count_out = path_wnull;
            return xr::Result::SUCCESS;
        }

        if *count_out != path_wnull {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if buf.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        copy_str_to_cchar_ptr::<{ xr::MAX_PATH_LENGTH }>(path, buf);
        log::debug!("path_to_string {xr_path:?}->{path}");

        Ok(())
    }))
}
