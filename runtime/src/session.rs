use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock, Mutex, atomic},
};

use ash::vk::Handle;

use crate::{
    error::{Error, Result, to_xr_result},
    event::{Event, schedule_event},
    loader::START_TIME,
    system::HMD_SYSTEM_ID,
    with_instance,
};

#[macro_export]
macro_rules! with_session {
    ($xr_session:expr, |$instance:ident| $expr:expr) => {{
        match $crate::session::get_simulated_session_cell($xr_session) {
            Ok(instance_ptr) => {
                let $instance = unsafe { &mut *instance_ptr };
                $expr
            }
            Err(err) => {
                log::error!("error: {err}");
                Err(openxr_sys::Result::ERROR_SESSION_LOST.into())
            }
        }
    }};
}

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

    to_xr_result(with_instance!(xr_instance, |instance| {
        let mut session_instances = INSTANCES.lock().unwrap();
        let next_id = INSTANCE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        session_instances.insert(
            next_id,
            UnsafeCell::new(
                match SimulatedSession::new(xr_instance.into_raw(), next_id, create_info) {
                    Ok(sess) => {
                        log::debug!("created: {:?}", &sess);
                        sess
                    }
                    Err(err) => return err.into(),
                },
            ),
        );

        *xr_session = xr::Session::from_raw(next_id);

        instance.set_session(next_id)
    }))
}

pub extern "system" fn destroy(xr_obj: xr::Session) -> xr::Result {
    if xr_obj == xr::Session::NULL {
        return xr::Result::ERROR_HANDLE_INVALID;
    }

    let instance_id = xr_obj.into_raw();

    log::debug!("destroyed session {instance_id} (todo)");
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

    to_xr_result(with_session!(xr_session, |session| {
        for i in 0..attach_info.count_action_sets {
            let item = unsafe { &*attach_info.action_sets.add(i as usize) };

            if let Err(err) = session.attach_action_set(item.into_raw()) {
                return err.into();
            }
        }
        Ok(())
    }))
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

    to_xr_result(with_session!(xr_session, |session| session.begin()))
}

pub extern "system" fn request_exit(xr_session: xr::Session) -> xr::Result {
    to_xr_result(with_session!(xr_session, |session| session.request_exit()))
}

pub extern "system" fn end(xr_session: xr::Session) -> xr::Result {
    to_xr_result(with_session!(xr_session, |session| session.end()))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimulatedSessionSpace {
    Reference,
    Action,
}

pub struct GraphicsBinding {
    pub instance: Arc<ash::Instance>,
    pub physical_device: ash::vk::PhysicalDevice,
    pub device: Arc<ash::Device>,
    pub queue_family_index: u32,
    pub queue_index: u32,
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
    instance_id: u64,
    id: u64,
    graphics_binding: GraphicsBinding,
    space_ids: HashMap<SimulatedSessionSpace, u64>,
    action_set_ids: HashSet<u64>,
    swapchain_ids: HashSet<u64>,
    state: xr::SessionState,
    is_running: bool,
}

static INSTANCE_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[inline]
pub fn get_simulated_session_cell(instance: xr::Session) -> Result<*mut SimulatedSession> {
    Ok(INSTANCES
        .lock()?
        .get(&instance.into_raw())
        .ok_or_else(|| Error::ExpectedSome("session does not exist".into()))?
        .get())
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

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn graphics_binding(&self) -> &GraphicsBinding {
        &self.graphics_binding
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
        self.space_ids.insert(space_type, space_id);
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

type SharedSimulatedSession = UnsafeCell<SimulatedSession>;

static INSTANCES: LazyLock<Mutex<HashMap<u64, SharedSimulatedSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
