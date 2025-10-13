use std::{ffi::c_char, ptr, time::Duration};

#[macro_export]
macro_rules! bind_api_fn {
    ($fn_type:ty, $fn_name:expr) => {{
        let fn_ptr: $fn_type = $fn_name;
        std::mem::transmute::<$fn_type, openxr_sys::pfn::VoidFunction>(fn_ptr)
    }};
}

pub fn copy_u8slice_to_cchar_arr<const MAX: usize>(src: &[u8], dst: &mut [c_char; MAX]) {
    if src.len() > MAX {
        panic!("src is too large");
    }

    unsafe { ptr::copy_nonoverlapping(src.as_ptr() as *const c_char, dst.as_mut_ptr(), src.len()) };
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
                ptr::copy_nonoverlapping(
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
