use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleA,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExA, DefWindowProcA, DispatchMessageA, GetClientRect,
            GetWindowLongPtrA, LoadCursorA, PeekMessageA, PostQuitMessage, RegisterClassExA,
            SetWindowLongPtrA, ShowWindow, TranslateMessage, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, PM_REMOVE, SW_HIDE, SW_SHOW, WM_CREATE,
            WM_DESTROY, WM_KEYDOWN, WM_QUIT, WNDCLASSEXA, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::util::{print_debug_string, AsCString};

pub struct Window {
    hwnd: HWND,
}

impl Window {
    fn new(
        title: impl Into<String>,
        window_size: (i32, i32),
    ) -> Result<Self, Box<dyn std::error::Error>> {
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

        let title = title.into();

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

        Ok(Self { hwnd })
    }

    fn on_key_down(&mut self) {
        print_debug_string("WINDOW: key down");
    }

    pub fn get_handle(&self) -> HWND {
        self.hwnd
    }

    pub fn get_physical_size(&self) -> (i32, i32) {
        let mut window_rect = RECT::default();
        if let Err(e) = unsafe { GetClientRect(self.hwnd, &mut window_rect) } {
            print_debug_string(&format!("failed to get client rect {e}"));
        }

        (
            window_rect.right - window_rect.left,
            window_rect.bottom - window_rect.top,
        )
    }

    pub fn set_visible(&self, visible: bool) {
        let show = if visible { SW_SHOW } else { SW_HIDE };
        let _ = unsafe { ShowWindow(self.hwnd, show) };
    }
}

pub struct App {}

impl App {
    pub fn init(
        title: impl Into<String>,
        window_size: (i32, i32),
    ) -> Result<(App, Window), Box<dyn std::error::Error>> {
        let app = App {};

        let window = Window::new(title, window_size)?;
        window.set_visible(true);

        Ok((app, window))
    }

    pub fn run(&mut self) -> bool {
        let mut running = true;
        let mut message = MSG::default();
        while running {
            if unsafe { PeekMessageA(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageA(&message);
                }

                if message.message == WM_QUIT {
                    running = false;
                    break;
                }
            } else {
                break;
            }
        }

        running
    }
}

fn window_wndproc(window: &mut Window, message: u32, wparam: WPARAM) -> bool {
    match message {
        // todo: handle window sizing, keys, etc.
        WM_KEYDOWN => {
            let w = wparam.0 as u8;
            print_debug_string(&format!("KEY DOWN: {w}"));
            window.on_key_down();
            true
        }

        _ => false,
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            let create_struct: &CREATESTRUCTA = unsafe { std::mem::transmute(lparam) };
            unsafe { SetWindowLongPtrA(hwnd, GWLP_USERDATA, create_struct.lpCreateParams as _) };
            LRESULT::default()
        }

        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT::default()
        }

        _ => {
            let user_data = unsafe { GetWindowLongPtrA(hwnd, GWLP_USERDATA) };
            let window = std::ptr::NonNull::<Window>::new(user_data as _);
            let handled =
                window.is_some_and(|mut w| window_wndproc(unsafe { w.as_mut() }, message, wparam));

            if handled {
                LRESULT::default()
            } else {
                unsafe { DefWindowProcA(hwnd, message, wparam, lparam) }
            }
        }
    }
}
