#![windows_subsystem = "windows"]

use std::ffi::CString;

use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::{Diagnostics::Debug::OutputDebugStringA, LibraryLoader::GetModuleHandleA},
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExA, DefWindowProcA, DispatchMessageA, LoadCursorA,
            PeekMessageA, PostQuitMessage, RegisterClassExA, ShowWindow, TranslateMessage,
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG, PM_REMOVE, SW_SHOW, WM_DESTROY,
            WM_QUIT, WNDCLASSEXA, WS_OVERLAPPEDWINDOW,
        },
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
        let message = s.as_c_string();
        unsafe {
            OutputDebugStringA(PCSTR(message.as_ptr() as _));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandLine {
    pub use_warp_device: bool,
}

pub fn build_command_line() -> CommandLine {
    let mut use_warp_device = false;

    for arg in std::env::args() {
        if arg.eq_ignore_ascii_case(("-warp")) || arg.eq_ignore_ascii_case("/warp") {
            use_warp_device = true;
        }
    }

    CommandLine { use_warp_device }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = unsafe { GetModuleHandleA(None) }?;

    let class_name = s!("LearnD3D12Class");

    let wc = WNDCLASSEXA {
        cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        hCursor: unsafe { LoadCursorA(None, PCSTR(IDC_ARROW.0 as _)) }?,
        lpszClassName: class_name,
        ..Default::default()
    };

    let command_line = build_command_line();

    let window_size = (800, 600);
    if unsafe { RegisterClassExA(&wc) } == 0 {
        panic!("LearnD3D12Class is already registered");
    }

    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: window_size.0,
        bottom: window_size.1,
    };
    unsafe { AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, false) }?;

    let mut title = "Hello Window Clear".to_string();
    if command_line.use_warp_device {
        title.push_str(" (WARP)");
    }

    let hwnd = unsafe {
        CreateWindowExA(
            Default::default(),
            class_name,
            PCSTR(title.as_c_string().as_ptr() as _),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            window_rect.right - window_rect.left,
            window_rect.bottom - window_rect.top,
            None, // No parent window.
            None, // No menus.
            instance,
            None, // No window data.
        )
    }?;

    if hwnd == HWND::default() {
        panic!("failed to create a window handle");
    }

    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

    let mut running = true;
    while running {
        let mut message = MSG::default();

        if unsafe { PeekMessageA(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageA(&message);
            }

            if message.message == WM_QUIT {
                running = false;
            }
        }
    }

    Ok(())
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT::default()
        }

        _ => unsafe { DefWindowProcA(hwnd, message, wparam, lparam) },
    }
}
