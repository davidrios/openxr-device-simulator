use std::{
    collections::{HashMap, HashSet},
    ffi::CStr,
    sync::atomic,
};

use crate::{prelude::*, utils::copy_str_to_cchar_arr};

static COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(1);

#[allow(dead_code)]
#[derive(Debug)]
pub enum InstanceState {
    Created,
    SessionCreated,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ActionBinding {
    pub(crate) action: u64,
    pub(crate) binding: u64,
}

impl ActionBinding {
    pub fn new(action: u64, binding: u64) -> Self {
        Self { action, binding }
    }
}

#[derive(Debug)]
pub struct SimulatedInstance {
    pub(crate) id: u64,
    pub(crate) state: InstanceState,
    pub(crate) session_id: Option<u64>,
    pub(crate) action_set_ids: HashSet<u64>,
    pub(crate) paths: HashMap<u64, String>,
    pub(crate) interaction_profile_bindings: HashMap<u64, Vec<ActionBinding>>,
}

impl SimulatedInstance {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            state: InstanceState::Created,
            session_id: None,
            action_set_ids: HashSet::new(),
            paths: HashMap::new(),
            interaction_profile_bindings: HashMap::new(),
        }
    }

    pub fn create(&mut self, create_info: &xr::InstanceCreateInfo) -> Result<()> {
        log::debug!("[{}]: create_info: {:?}", self.id, create_info);
        Err(xr::Result::ERROR_RUNTIME_UNAVAILABLE.into())
    }

    pub fn get_properties(&mut self, properties: &mut xr::InstanceProperties) -> Result<()> {
        properties.runtime_version = xr::Version::new(0, 0, 1);
        copy_str_to_cchar_arr("openxr-device-simulator", &mut properties.runtime_name);
        log::debug!("[{}]: get_properties: {:?}", self.id, properties);

        Ok(())
    }

    pub fn register_path(&mut self, path: &CStr) -> Result<u64> {
        let new_id = COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
        self.paths.insert(new_id, path.to_str()?.into());
        log::debug!(
            "[{}] registered path {} at {}",
            self.id,
            &self.paths[&new_id],
            new_id
        );
        Ok(new_id)
    }

    pub fn get_path_string(&mut self, path_id: u64) -> Result<&String> {
        if let Some(path) = self.paths.get(&path_id) {
            Ok(path)
        } else {
            Err(xr::Result::ERROR_PATH_INVALID.into())
        }
    }

    pub fn set_session(&mut self, session_id: u64) -> Result<()> {
        if let InstanceState::Created = self.state {
            self.session_id = Some(session_id);
            self.state = InstanceState::SessionCreated;
            Ok(())
        } else {
            Err(format!("unexpected state: {:?}", self.state)
                .as_str()
                .into())
        }
    }

    /// Called when the instance's session is destroyed, so a later
    /// `xrCreateSession` on the same instance (a valid, real-world pattern —
    /// e.g. Godot's OpenXR module creates and destroys a probe session
    /// before the real one) isn't rejected by a state machine stuck at
    /// `SessionCreated`.
    pub fn clear_session(&mut self) {
        self.session_id = None;
        self.state = InstanceState::Created;
    }

    /// Action sets are an instance-level concept in OpenXR — apps
    /// legitimately create them right after `xrCreateInstance`, well before
    /// any session exists (Godot does this; `hello_xr`/`bevy_oxr` happen to
    /// create their session first, which is also valid but had made this
    /// look like a hard ordering requirement). No session-state gate here.
    pub fn add_action_set(&mut self, action_set_id: u64) -> Result<()> {
        self.action_set_ids.insert(action_set_id);
        Ok(())
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
