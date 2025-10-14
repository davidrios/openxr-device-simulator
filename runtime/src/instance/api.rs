use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{CStr, c_char},
    sync::{LazyLock, Mutex, atomic},
};

use crate::{
    error::{Error, IntoXrResult, Result},
    event::create_queue,
    utils::{copy_str_to_cchar_ptr, copy_u8slice_to_cchar_arr, with_obj_instance},
};

use super::obj::SimulatedInstance;

const SUPPORTED_EXTS: &[(&[u8], u32)] = &[(
    xr::KHR_VULKAN_ENABLE_EXTENSION_NAME,
    xr::KHR_vulkan_enable_SPEC_VERSION,
)];

pub extern "system" fn enumerate_extension_properties(
    layer_name: *const c_char,
    capacity_in: u32,
    count_out: *mut u32,
    properties: *mut xr::ExtensionProperties,
) -> xr::Result {
    let layer_name_str = if layer_name.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(layer_name).to_str().unwrap_or("") })
    };

    log::debug!(
        "enumerate extension props: name={layer_name_str:?}, cap_in={capacity_in}, cap_out={}",
        unsafe { *count_out }
    );

    if capacity_in == 0 {
        unsafe { *count_out = SUPPORTED_EXTS.len() as u32 };
        return xr::Result::SUCCESS;
    }

    if unsafe { *count_out } as usize != SUPPORTED_EXTS.len() {
        return xr::Result::ERROR_SIZE_INSUFFICIENT;
    }

    for (idx, ext) in SUPPORTED_EXTS.iter().enumerate() {
        let prop = unsafe { &mut *properties.add(idx) };
        copy_u8slice_to_cchar_arr(ext.0, &mut prop.extension_name);
        prop.extension_version = ext.1;
    }

    xr::Result::SUCCESS
}

pub extern "system" fn create(
    create_info: *const xr::InstanceCreateInfo,
    xr_instance: *mut xr::Instance,
) -> xr::Result {
    if xr_instance.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
    INSTANCES
        .lock()
        .unwrap()
        .insert(next_id, UnsafeCell::new(SimulatedInstance::new(next_id)));

    unsafe {
        *xr_instance = xr::Instance::from_raw(next_id);
    }

    if let Err(err) = create_queue(next_id) {
        return err.into();
    }

    log::debug!("created new instance: {next_id}, {:?}", unsafe {
        *create_info
    });

    xr::Result::SUCCESS
}

pub extern "system" fn destroy(xr_obj: xr::Instance) -> xr::Result {
    if xr_obj == xr::Instance::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    if INSTANCES
        .lock()
        .expect("couldn't acquire instances")
        .remove(&instance_id)
        .is_some()
    {
        log::debug!("destroyed {instance_id}");
    } else {
        log::debug!("instance {instance_id} not found");
    }

    xr::Result::SUCCESS
}

pub extern "system" fn create_from_instance(
    create_info: *const xr::InstanceCreateInfo,
    xr_instance: *mut xr::Instance,
) -> xr::Result {
    if create_info.is_null() || xr_instance.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_instance) = unsafe { (&*create_info, &mut *xr_instance) };

    if create_info.ty != xr::StructureType::INSTANCE_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance((*xr_instance).into_raw(), |instance| {
        instance.create(create_info)
    })
    .into_xr_result()
}

pub extern "system" fn get_properties(
    xr_instance: xr::Instance,
    properties: *mut xr::InstanceProperties,
) -> xr::Result {
    if properties.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let properties = unsafe { &mut *properties };
    if properties.ty != xr::StructureType::INSTANCE_PROPERTIES {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance(xr_instance.into_raw(), |instance| {
        instance.get_properties(properties)
    })
    .into_xr_result()
}

pub extern "system" fn result_to_string(
    xr_instance: xr::Instance,
    xr_result: xr::Result,
    buf: *mut c_char,
) -> xr::Result {
    if buf.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance(xr_instance.into_raw(), |_instance| {
        let result_int = xr_result.into_raw();
        let res = if result_int >= 0 {
            format!("XR_UNKNOWN_SUCCESS_{result_int}")
        } else {
            format!("XR_UNKNOWN_FAILURE_{result_int}")
        };
        copy_str_to_cchar_ptr::<{ xr::MAX_RESULT_STRING_SIZE }>(&res, buf);
        log::debug!("result_to_string {xr_result}->{res}");
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn structure_type_to_string(
    xr_instance: xr::Instance,
    structure_type: xr::StructureType,
    buf: *mut c_char,
) -> xr::Result {
    if buf.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    with_instance(xr_instance.into_raw(), |_instance| {
        let result_int = structure_type.into_raw();
        let res = format!("XR_UNKNOWN_STRUCTURE_TYPE_{result_int}");
        copy_str_to_cchar_ptr::<{ xr::MAX_RESULT_STRING_SIZE }>(&res, buf);
        log::debug!("structure_type_to_string {structure_type:?}->{res}");
        Ok(())
    })
    .into_xr_result()
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedInstance = UnsafeCell<SimulatedInstance>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedInstance>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_instance<T, F>(xr_instance_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedInstance) -> Result<T>,
{
    match with_obj_instance(&INSTANCES, xr_instance_id, f) {
        Ok(res) => Ok(res),
        Err(err) => match err {
            Error::ExpectedSome(err) => {
                log::error!("error: {err}");
                Err(openxr_sys::Result::ERROR_INSTANCE_LOST.into())
            }
            _ => Err(err),
        },
    }
}
