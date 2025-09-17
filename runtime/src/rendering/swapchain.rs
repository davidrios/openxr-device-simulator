use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::{LazyLock, Mutex, atomic},
};

use openxr_sys as xr;

use crate::{
    error::{Error, Result, to_xr_result},
    with_session,
};

#[macro_export]
macro_rules! with_swapchain {
    ($xr_obj:expr, |$instance:ident| $expr:expr) => {{
        match $crate::rendering::swapchain::get_simulated_swapchain_cell($xr_obj) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => Err(err),
        }
    }};
}

const SUPPORTED_SWAPCHAIN_FORMATS: &[i64] = &[
    ash::vk::Format::R8G8B8_UNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8_SNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8_UINT.as_raw() as i64,
    ash::vk::Format::R8G8B8_SINT.as_raw() as i64,
    ash::vk::Format::R8G8B8_SRGB.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_UNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SNORM.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_UINT.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SINT.as_raw() as i64,
    ash::vk::Format::R8G8B8A8_SRGB.as_raw() as i64,
];

pub extern "system" fn enumerate_formats(
    xr_session: xr::Session,
    capacity_in: u32,
    count_out: *mut u32,
    formats: *mut i64,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_session!(xr_session, |_session| {
        if capacity_in == 0 {
            *count_out = SUPPORTED_SWAPCHAIN_FORMATS.len() as u32;
            return xr::Result::SUCCESS;
        }

        if *count_out != SUPPORTED_SWAPCHAIN_FORMATS.len() as u32 {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if formats.is_null() {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        unsafe {
            for (i, format) in SUPPORTED_SWAPCHAIN_FORMATS.iter().enumerate() {
                *formats.add(i) = *format;
            }
        }

        Ok(())
    }))
}

pub extern "system" fn create(
    xr_session: xr::Session,
    create_info: *const xr::SwapchainCreateInfo,
    xr_swapchain: *mut xr::Swapchain,
) -> xr::Result {
    if create_info.is_null() || xr_swapchain.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_swapchain) = unsafe { (&*create_info, &mut *xr_swapchain) };

    if create_info.ty != xr::StructureType::SWAPCHAIN_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    to_xr_result(with_session!(xr_session, |session| {
        let mut swapchain_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        swapchain_instances.insert(
            next_id,
            UnsafeCell::new(
                match SimulatedSwapchain::new(xr_session.into_raw(), next_id, create_info) {
                    Ok(set) => set,
                    Err(err) => return err.into(),
                },
            ),
        );

        log::debug!("create swapchain: {:?}", unsafe {
            &*swapchain_instances[&next_id].get()
        });

        *xr_swapchain = xr::Swapchain::from_raw(next_id);

        session.add_swapchain(next_id)
    }))
}

pub extern "system" fn enumerate_images(
    xr_swapchain: xr::Swapchain,
    capacity_in: u32,
    count_out: *mut u32,
    images: *mut xr::SwapchainImageBaseHeader,
) -> xr::Result {
    let count_out = unsafe { &mut *count_out };

    to_xr_result(with_swapchain!(xr_swapchain, |swapchain| {
        if capacity_in == 0 {
            *count_out = swapchain.array_size;
            return xr::Result::SUCCESS;
        }

        if *count_out != swapchain.array_size {
            return xr::Result::ERROR_SIZE_INSUFFICIENT;
        }

        if images.is_null()
            || !matches!(
                unsafe { *images }.ty,
                xr::StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR
            )
        {
            return xr::Result::ERROR_VALIDATION_FAILURE;
        }

        log::debug!("enumerate images");

        for i in 0..(*count_out as usize) {
            let item = unsafe { &*(images.add(i) as *mut xr::SwapchainImageVulkanKHR) };
            log::debug!("{item:?}");
        }

        Ok(())
    }))
}

#[derive(Debug)]
pub struct SimulatedSwapchain {
    session_id: u64,
    id: u64,
    create_flags: xr::SwapchainCreateFlags,
    usage_flags: xr::SwapchainUsageFlags,
    format: ash::vk::Format,
    sample_count: u32,
    width: u32,
    height: u32,
    face_count: u32,
    array_size: u32,
    mip_count: u32,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

impl SimulatedSwapchain {
    pub fn new(session_id: u64, id: u64, create_info: &xr::SwapchainCreateInfo) -> Result<Self> {
        if !SUPPORTED_SWAPCHAIN_FORMATS.contains(&create_info.format) {
            return Err(xr::Result::ERROR_SWAPCHAIN_FORMAT_UNSUPPORTED.into());
        }

        let format = ash::vk::Format::from_raw(create_info.format as i32);

        Ok(Self {
            session_id,
            id,
            create_flags: create_info.create_flags,
            usage_flags: create_info.usage_flags,
            format,
            sample_count: create_info.sample_count,
            width: create_info.width,
            height: create_info.height,
            face_count: create_info.face_count,
            array_size: create_info.array_size,
            mip_count: create_info.mip_count,
        })
    }
}

type SharedSimulatedSwapchain = UnsafeCell<SimulatedSwapchain>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSwapchain>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn get_simulated_swapchain_cell(instance: xr::Swapchain) -> Result<*mut SimulatedSwapchain> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("swapchain does not exist".into()))?
        .get())
}
