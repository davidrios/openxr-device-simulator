use openxr_sys as xr;

use crate::with_instance;

pub extern "system" fn poll(
    xr_instance: xr::Instance,
    event_data: *mut xr::EventDataBuffer,
) -> xr::Result {
    if event_data.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let _event_data = unsafe { &mut *event_data };

    with_instance!(xr_instance, |_instance| {
        // log::debug!("poll event");
        xr::Result::SUCCESS
    })
}
