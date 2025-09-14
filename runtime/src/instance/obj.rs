use std::{collections::HashMap, ffi::CStr, sync::atomic};

use openxr_sys as xr;

use crate::{error::Result, utils::copy_cstr_to_i8};

static COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[derive(Debug)]
pub enum InstanceState {
    Created,
    SessionCreated,
    ActionSetCreated,
}

#[derive(Debug)]
pub struct ActionBinding {
    action: u64,
    binding: u64,
}

impl ActionBinding {
    pub fn new(action: u64, binding: u64) -> Self {
        Self { action, binding }
    }
}

#[derive(Debug)]
pub struct SimulatedInstance {
    id: u64,
    state: InstanceState,
    session_id: Option<u64>,
    action_set_id: Option<u64>,
    paths: HashMap<u64, String>,
    interaction_profile_bindings: HashMap<u64, Vec<ActionBinding>>,
}

impl SimulatedInstance {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            state: InstanceState::Created,
            session_id: None,
            action_set_id: None,
            paths: HashMap::new(),
            interaction_profile_bindings: HashMap::new(),
        }
    }

    pub fn create(&mut self, create_info: &xr::InstanceCreateInfo) -> xr::Result {
        log::debug!("[{}]: create_info: {:?}", self.id, create_info);
        xr::Result::ERROR_RUNTIME_UNAVAILABLE
    }

    pub fn get_properties(&mut self, properties: &mut xr::InstanceProperties) -> xr::Result {
        properties.runtime_version = xr::Version::new(0, 0, 1);
        copy_cstr_to_i8(
            "openxr-device-simulator".as_bytes(),
            &mut properties.runtime_name,
        );
        log::debug!("[{}]: get_properties: {:?}", self.id, properties);

        xr::Result::SUCCESS
    }

    pub fn register_path(&mut self, path: &CStr) -> Result<u64> {
        let new_id = COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        self.paths.insert(new_id, path.to_string_lossy().into());
        log::debug!(
            "[{}] registered path {} at {}",
            self.id,
            &self.paths[&new_id],
            new_id
        );
        Ok(new_id)
    }

    pub fn set_session(&mut self, session_id: u64) -> xr::Result {
        if let InstanceState::Created = self.state {
            self.session_id = Some(session_id);
            self.state = InstanceState::SessionCreated;
            xr::Result::SUCCESS
        } else {
            log::error!("unexpected state: {:?}", self.state);
            xr::Result::ERROR_RUNTIME_FAILURE
        }
    }

    pub fn set_action_set(&mut self, action_set_id: u64) -> xr::Result {
        if let InstanceState::SessionCreated = self.state {
            self.action_set_id = Some(action_set_id);
            self.state = InstanceState::ActionSetCreated;
            xr::Result::SUCCESS
        } else {
            log::error!("unexpected state: {:?}", self.state);
            xr::Result::ERROR_RUNTIME_FAILURE
        }
    }

    pub fn set_interaction_profile_bindings(
        &mut self,
        interaction_profile: u64,
        bindings: Vec<ActionBinding>,
    ) -> Result<()> {
        self.interaction_profile_bindings
            .insert(interaction_profile, bindings);
        log::debug!(
            "set interaction profile bindings {:?}",
            &self.interaction_profile_bindings[&interaction_profile]
        );
        Ok(())
    }
}
