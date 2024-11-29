use std::ffi::CString;

use windows::{core::PCSTR, Win32::System::Diagnostics::Debug::OutputDebugStringA};

pub trait AsCString {
    fn as_c_string(&self) -> CString;
}

impl AsCString for String {
    fn as_c_string(&self) -> CString {
        CString::new(self.clone()).unwrap_or_default()
    }
}

impl AsCString for &str {
    fn as_c_string(&self) -> CString {
        self.to_string().as_c_string()
    }
}

pub fn print_debug_string(s: &str) {
    if cfg!(debug_assertions) {
        let message = s.as_c_string();
        unsafe {
            OutputDebugStringA(PCSTR(message.as_ptr() as _));
        }
    }
}
