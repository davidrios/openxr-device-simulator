use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock, Mutex, atomic},
};

use ash::vk::Handle;

use crate::{
    error::{Error, IntoXrResult, Result},
    event::{Event, schedule_event},
    instance::api::with_instance,
    loader::START_TIME,
    system::HMD_SYSTEM_ID,
    utils::with_obj_instance,
};

pub extern "system" fn create(
    xr_instance: xr::Instance,
    create_info: *const xr::SessionCreateInfo,
    xr_session: *mut xr::Session,
) -> xr::Result {
    if create_info.is_null() || xr_session.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let (create_info, xr_session) = unsafe { (&*create_info, &mut *xr_session) };

    if create_info.ty != xr::StructureType::SESSION_CREATE_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if create_info.system_id != xr::SystemId::from_raw(HMD_SYSTEM_ID) {
        return xr::Result::ERROR_SYSTEM_INVALID;
    }

    log::debug!("create: {:?}", create_info);

    with_instance(xr_instance.into_raw(), |instance| {
        let mut session_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        session_instances.insert(
            next_id,
            UnsafeCell::new(SimulatedSession::new(
                xr_instance.into_raw(),
                next_id,
                create_info,
            )?),
        );

        log::debug!("created: {:?}", unsafe {
            &*session_instances[&next_id].get()
        });

        *xr_session = xr::Session::from_raw(next_id);

        instance.set_session(next_id)
    })
    .into_xr_result()
}

pub extern "system" fn destroy(xr_obj: xr::Session) -> xr::Result {
    if xr_obj == xr::Session::NULL {
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

pub extern "system" fn attach_action_sets(
    xr_session: xr::Session,
    attach_info: *const xr::SessionActionSetsAttachInfo,
) -> xr::Result {
    if attach_info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let attach_info = unsafe { &*attach_info };

    if attach_info.ty != xr::StructureType::SESSION_ACTION_SETS_ATTACH_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if attach_info.count_action_sets == 0 || attach_info.action_sets.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let action_sets: &[xr::ActionSet] = unsafe {
        std::slice::from_raw_parts(
            attach_info.action_sets,
            attach_info.count_action_sets as usize,
        )
    };

    with_session(xr_session.into_raw(), |session| {
        for item in action_sets {
            session.attach_action_set(item.into_raw())?;
        }
        Ok(())
    })
    .into_xr_result()
}

pub extern "system" fn begin(
    xr_session: xr::Session,
    begin_info: *const xr::SessionBeginInfo,
) -> xr::Result {
    if begin_info.is_null() {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    let begin_info = unsafe { &*begin_info };

    if begin_info.ty != xr::StructureType::SESSION_BEGIN_INFO {
        return xr::Result::ERROR_VALIDATION_FAILURE;
    }

    if begin_info.primary_view_configuration_type != xr::ViewConfigurationType::PRIMARY_STEREO {
        return xr::Result::ERROR_VIEW_CONFIGURATION_TYPE_UNSUPPORTED;
    }

    with_session(xr_session.into_raw(), |session| session.begin()).into_xr_result()
}

pub extern "system" fn request_exit(xr_session: xr::Session) -> xr::Result {
    with_session(xr_session.into_raw(), |session| session.request_exit()).into_xr_result()
}

pub extern "system" fn end(xr_session: xr::Session) -> xr::Result {
    with_session(xr_session.into_raw(), |session| session.end()).into_xr_result()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulatedSessionSpace {
    Reference,
    Action,
}

pub struct GraphicsBinding {
    pub(crate) instance: Arc<ash::Instance>,
    pub(crate) physical_device: ash::vk::PhysicalDevice,
    pub(crate) device: Arc<ash::Device>,
    pub(crate) queue_family_index: u32,
    pub(crate) queue_index: u32,
}

impl std::fmt::Debug for GraphicsBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphicsBinding")
            .field("instance", &self.instance.handle())
            .field("physical_device", &self.physical_device)
            .field("device", &self.device.handle())
            .field("queue_family_index", &self.queue_family_index)
            .field("queue_index", &self.queue_index)
            .finish()
    }
}

impl TryFrom<&xr::GraphicsBindingVulkanKHR> for GraphicsBinding {
    type Error = Error;

    fn try_from(value: &xr::GraphicsBindingVulkanKHR) -> Result<Self> {
        let entry = match unsafe { ash::Entry::load() } {
            Ok(e) => e,
            Err(e) => {
                log::error!("failed to load Vulkan loader: {}", e);
                return Err(xr::Result::ERROR_RUNTIME_FAILURE.into());
            }
        };

        unsafe {
            let vk_instance = ash::vk::Instance::from_raw(value.instance as u64);
            let instance = Arc::new(ash::Instance::load(entry.static_fn(), vk_instance));

            let vk_device = ash::vk::Device::from_raw(value.device as u64);
            let device = Arc::new(ash::Device::load(instance.fp_v1_0(), vk_device));

            Ok(Self {
                instance,
                physical_device: ash::vk::PhysicalDevice::from_raw(value.physical_device as u64),
                device,
                queue_family_index: value.queue_family_index,
                queue_index: value.queue_index,
            })
        }
    }
}

#[derive(Debug)]
pub struct SimulatedSession {
    pub(crate) instance_id: u64,
    pub(crate) id: u64,
    pub(crate) graphics_binding: GraphicsBinding,
    pub(crate) space_ids: HashMap<u64, SimulatedSessionSpace>,
    pub(crate) action_set_ids: HashSet<u64>,
    pub(crate) swapchain_ids: HashSet<u64>,
    pub(crate) state: xr::SessionState,
    pub(crate) is_running: bool,
}

impl SimulatedSession {
    pub fn new(instance_id: u64, id: u64, create_info: &xr::SessionCreateInfo) -> Result<Self> {
        if create_info.next.is_null() {
            return Err(xr::Result::ERROR_GRAPHICS_DEVICE_INVALID.into());
        }

        let graphics_binding =
            unsafe { &*(create_info.next as *const xr::GraphicsBindingVulkanKHR) };

        if graphics_binding.ty != xr::StructureType::GRAPHICS_BINDING_VULKAN_KHR {
            return Err(xr::Result::ERROR_GRAPHICS_DEVICE_INVALID.into());
        }

        let sess = Self {
            instance_id,
            id,
            graphics_binding: graphics_binding.try_into()?,
            space_ids: HashMap::new(),
            action_set_ids: HashSet::new(),
            swapchain_ids: HashSet::new(),
            state: xr::SessionState::IDLE,
            is_running: false,
        };

        schedule_event(
            instance_id,
            &Event::SessionStateChanged {
                session: xr::Session::from_raw(sess.id),
                state: sess.state,
                time: START_TIME.elapsed().into(),
            },
        )?;

        Ok(sess)
    }

    pub fn check_ready(&mut self) -> Result<()> {
        if let xr::SessionState::IDLE = self.state {
            if !self.space_ids.is_empty()
                && !self.swapchain_ids.is_empty()
                && !self.action_set_ids.is_empty()
            {
                self.state = xr::SessionState::READY;
                schedule_event(
                    self.instance_id,
                    &Event::SessionStateChanged {
                        session: xr::Session::from_raw(self.id),
                        state: self.state,
                        time: START_TIME.elapsed().into(),
                    },
                )?;
            }
        }

        Ok(())
    }

    pub fn set_space(&mut self, space_type: SimulatedSessionSpace, space_id: u64) -> Result<()> {
        self.space_ids.insert(space_id, space_type);
        self.check_ready()?;

        Ok(())
    }

    pub fn attach_action_set(&mut self, action_set_id: u64) -> Result<()> {
        if !self.action_set_ids.insert(action_set_id) {
            Err(xr::Result::ERROR_ACTIONSETS_ALREADY_ATTACHED.into())
        } else {
            log::debug!("attached action set {action_set_id}");
            self.check_ready()?;

            Ok(())
        }
    }

    pub fn has_attached_action_set(&self, action_set_id: u64) -> bool {
        self.action_set_ids.contains(&action_set_id)
    }

    pub fn add_swapchain(&mut self, swapchain_id: u64) -> Result<()> {
        if !self.swapchain_ids.insert(swapchain_id) {
            Err(xr::Result::ERROR_ACTIONSETS_ALREADY_ATTACHED.into())
        } else {
            log::debug!("attached swapchain {swapchain_id}");
            self.check_ready()?;

            Ok(())
        }
    }

    pub fn begin(&mut self) -> Result<()> {
        if self.is_running {
            return Err(xr::Result::ERROR_SESSION_RUNNING.into());
        }

        if !matches!(self.state, xr::SessionState::READY) {
            return Err(xr::Result::ERROR_SESSION_NOT_READY.into());
        }

        self.is_running = true;
        log::debug!("{}: session began", self.id);
        Ok(())
    }

    pub fn is_focused(&self) -> bool {
        matches!(self.state, xr::SessionState::FOCUSED)
    }

    pub fn request_exit(&mut self) -> Result<()> {
        if !self.is_running {
            return Err(xr::Result::ERROR_SESSION_NOT_RUNNING.into());
        }

        self.state = xr::SessionState::STOPPING;
        schedule_event(
            self.instance_id,
            &Event::SessionStateChanged {
                session: xr::Session::from_raw(self.id),
                state: self.state,
                time: START_TIME.elapsed().into(),
            },
        )?;

        Ok(())
    }

    pub fn end(&mut self) -> Result<()> {
        if !self.is_running {
            return Err(xr::Result::ERROR_SESSION_NOT_RUNNING.into());
        }

        if !matches!(self.state, xr::SessionState::STOPPING) {
            return Err(xr::Result::ERROR_SESSION_NOT_STOPPING.into());
        }

        self.is_running = false;
        self.state = xr::SessionState::IDLE;
        schedule_event(
            self.instance_id,
            &Event::SessionStateChanged {
                session: xr::Session::from_raw(self.id),
                state: self.state,
                time: START_TIME.elapsed().into(),
            },
        )?;

        // self.check_ready()?;

        Ok(())
    }
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

type SharedSimulatedSession = UnsafeCell<SimulatedSession>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn with_session<T, F>(xr_obj_id: u64, f: F) -> Result<T>
where
    F: FnMut(&mut SimulatedSession) -> Result<T>,
{
    match with_obj_instance(&INSTANCES, xr_obj_id, f) {
        Ok(res) => Ok(res),
        Err(err) => match err {
            Error::ExpectedSome(err) => {
                log::error!("error: {err}");
                Err(openxr_sys::Result::ERROR_SESSION_LOST.into())
            }
            _ => Err(err),
        },
    }
}
