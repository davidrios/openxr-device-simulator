use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{CStr, c_char},
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result, to_xr_result},
    event::create_queue,
    utils::copy_cstr_to_i8,
};

#[macro_export]
macro_rules! with_instance {
    ($xr_instance:expr, |$instance:ident| $expr:expr) => {{
        if $xr_instance == xr::Instance::NULL {
            Err(xr::Result::ERROR_HANDLE_INVALID.into())
        } else {
            match $crate::instance::api::get_simulated_instance_cell($xr_instance) {
                Ok(instance_ptr) => {
                    let $instance = unsafe { &mut *instance_ptr };
                    $expr
                }
                Err(err) => {
                    log::error!("error: {err}");
                    Err(openxr_sys::Result::ERROR_INSTANCE_LOST.into())
                }
            }
        }
    }};
}

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
        copy_cstr_to_i8(ext.0, &mut prop.extension_name);
        prop.extension_version = ext.1;
    }

    xr::Result::SUCCESS
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedInstance = UnsafeCell<SimulatedInstance>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedInstance>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

#[inline]
pub fn get_simulated_instance_cell(instance: xr::Instance) -> Result<*mut SimulatedInstance> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("instance does not exist".into()))?
        .get())
}

pub extern "system" fn destroy(xr_obj: xr::Instance) -> xr::Result {
    if xr_obj == xr::Instance::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed {instance_id} (todo)");
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

    to_xr_result(with_instance!(*xr_instance, |instance| instance.create(create_info)))
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

    to_xr_result(with_instance!(xr_instance, |instance| instance.get_properties(properties)))
}
