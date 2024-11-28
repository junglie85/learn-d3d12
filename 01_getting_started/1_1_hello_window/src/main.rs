#![windows_subsystem = "windows"]

use std::ffi::CString;

use windows::{
    core::{s, PCSTR},
    Win32::{
        System::Diagnostics::Debug::OutputDebugStringA,
        UI::WindowsAndMessaging::{MessageBoxA, MB_OK},
    },
};

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
        unsafe {
            let message = s.as_c_string();
            OutputDebugStringA(PCSTR(message.as_ptr() as _));
        }
    }
}

fn main() {
    let message = "Hello, D3D12!".to_string();
    let m = message.as_c_string();
    unsafe {
        MessageBoxA(None, PCSTR(m.as_ptr() as _), s!("Greetings"), MB_OK);
    }

    print_debug_string(&message);
}
