use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::c_char,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use crate::prelude::*;

#[macro_export]
macro_rules! bind_api_fn {
    ($fn_type:ty, $fn_name:expr) => {{
        let fn_ptr: $fn_type = $fn_name;
        std::mem::transmute::<$fn_type, openxr_sys::pfn::VoidFunction>(fn_ptr)
    }};
}

#[inline]
pub fn with_obj_instance<T, U, F>(
    instances: &LazyLock<Mutex<HashMap<u64, UnsafeCell<U>>>>,
    obj_id: u64,
    mut f: F,
) -> Result<T>
where
    F: FnMut(&mut U) -> Result<T>,
{
    if obj_id == 0 {
        return Err(xr::Result::ERROR_HANDLE_INVALID.into());
    }

    f(unsafe {
        &mut *instances
            .lock()?
            .get(&obj_id)
            .ok_or_else(|| Error::ExpectedSome(format!("obj instance {obj_id} does not exist")))?
            .get()
    })
}

pub fn copy_u8slice_to_cchar_arr<const MAX: usize>(src: &[u8], dst: &mut [c_char; MAX]) {
    if src.len() > MAX {
        panic!("src is too large");
    }

    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst.as_mut_ptr(), src.len())
    };
}

pub fn copy_str_to_cchar_arr<const MAX: usize>(src: &str, dst: &mut [c_char; MAX]) {
    copy_str_to_cchar_ptr::<MAX>(src, dst as *mut c_char);
}

pub fn copy_str_to_cchar_ptr<const MAX: usize>(src: &str, dst: *mut c_char) {
    if src.len() + 1 > MAX {
        panic!("src is too large");
    }
    copy_u8slice_to_cchar_arr(src.as_bytes(), unsafe { &mut *(dst as *mut [c_char; MAX]) });
    unsafe {
        *dst.add(src.len()) = 0;
    }
}

pub struct ExtList<'a> {
    exts: Vec<&'a [u8]>,
}

impl<'a> ExtList<'a> {
    pub fn new(exts: Vec<&'a [u8]>) -> Self {
        Self { exts }
    }

    pub fn len(&self) -> usize {
        let mut size = 0;
        for i in 0..self.exts.len() {
            size += self.exts[i].len() + 1;
        }
        size
    }

    pub fn copy_to_cchar_ptr(&self, buffer: *mut c_char) {
        unsafe {
            let mut offset = 0;
            for i in 0..self.exts.len() {
                let src = self.exts[i];
                std::ptr::copy_nonoverlapping(
                    src.as_ptr() as *const c_char,
                    buffer.add(offset),
                    src.len(),
                );
                offset += src.len();
                *buffer.add(offset) = if i == self.exts.len() - 1 {
                    0
                } else {
                    ' ' as i8
                };
                offset += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MyTime(xr::Time);

impl From<MyTime> for xr::Time {
    fn from(value: MyTime) -> Self {
        value.0
    }
}

impl From<Duration> for MyTime {
    fn from(value: Duration) -> Self {
        Self(xr::Time::from_nanos(value.as_nanos().try_into().unwrap()))
    }
}

pub fn create_identity_pose() -> xr::Posef {
    xr::Posef {
        orientation: xr::Quaternionf {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        position: xr::Vector3f {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    }
}

/// Rotate a vector from a local frame into its parent frame: v' = q v q*
pub fn rotate_vec3(q: &xr::Quaternionf, v: xr::Vector3f) -> xr::Vector3f {
    let tx = 2.0 * (q.y * v.z - q.z * v.y);
    let ty = 2.0 * (q.z * v.x - q.x * v.z);
    let tz = 2.0 * (q.x * v.y - q.y * v.x);
    xr::Vector3f {
        x: v.x + q.w * tx + q.y * tz - q.z * ty,
        y: v.y + q.w * ty + q.z * tx - q.x * tz,
        z: v.z + q.w * tz + q.x * ty - q.y * tx,
    }
}

/// Hamilton product a * b: the rotation that applies b first, then a.
pub fn mul_quat(a: &xr::Quaternionf, b: &xr::Quaternionf) -> xr::Quaternionf {
    xr::Quaternionf {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

/// Compose a pose expressed in `parent`'s local frame with `parent` itself,
/// giving the pose in `parent`'s parent frame.
pub fn compose_poses(parent: xr::Posef, local: xr::Posef) -> xr::Posef {
    xr::Posef {
        orientation: mul_quat(&parent.orientation, &local.orientation),
        position: {
            let offset = rotate_vec3(&parent.orientation, local.position);
            xr::Vector3f {
                x: parent.position.x + offset.x,
                y: parent.position.y + offset.y,
                z: parent.position.z + offset.z,
            }
        },
    }
}
