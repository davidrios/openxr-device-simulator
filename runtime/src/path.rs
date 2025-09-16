use std::ffi::{CStr, c_char};

use openxr_sys as xr;

use crate::{error::to_xr_result, with_instance};

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
