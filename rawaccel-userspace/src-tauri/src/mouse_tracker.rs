use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RIDEV_INPUTSINK, RID_INPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, HWND_MESSAGE,
    MSG, WNDCLASSW,
};

#[derive(Clone, serde::Serialize)]
pub struct MouseSpeed {
    pub speed_x: f64,
    pub speed_y: f64,
    pub speed_whole: f64,
}

struct MouseState {
    dx: f64,
    dy: f64,
}

static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState { dx: 0.0, dy: 0.0 });

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    const WM_INPUT: u32 = 0x00FF;
    if msg == WM_INPUT {
        if let Some((x, y)) = read_mouse(HRAWINPUT(lparam.0 as _)) {
            if let Ok(mut state) = MOUSE_STATE.lock() {
                state.dx += x as f64;
                state.dy += y as f64;
            }
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn read_mouse(handle: HRAWINPUT) -> Option<(i32, i32)> {
    let mut size = 0u32;
    let header = std::mem::size_of::<RAWINPUTHEADER>() as u32;
    // With no output buffer, success is zero and size is the required byte count.
    if unsafe { GetRawInputData(handle, RID_INPUT, None, &mut size, header) } != 0 || size < header
    {
        return None;
    }
    let requested = size;
    let mut buffer = crate::input_buffers::InputBuffer::new(size)?;
    let returned = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            Some(buffer.as_mut_ptr()),
            &mut size,
            header,
        )
    };
    if returned == u32::MAX || returned > requested || size > requested || returned != size {
        return None;
    }
    crate::input_buffers::relative_mouse(buffer.bytes(returned)?)
}

pub fn start(app_handle: AppHandle) {
    // 1. Thread to pump messages and collect raw input
    thread::spawn(move || {
        unsafe {
            let h_instance = GetModuleHandleW(None).unwrap();
            let class_name = windows::core::w!("RawAccelMouseTracker");

            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: h_instance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };

            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                Default::default(),
                class_name,
                windows::core::w!("RawAccelMouseTrackerWindow"),
                Default::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(h_instance.into()),
                None,
            )
            .unwrap();

            // Register for raw input
            let rid = RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic desktop controls
                usUsage: 0x02,     // Mouse
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            };

            if let Err(e) =
                RegisterRawInputDevices(&[rid], std::mem::size_of::<RAWINPUTDEVICE>() as u32)
            {
                eprintln!("Failed to register raw input devices: {:?}", e);
                return;
            }

            // Run message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                DispatchMessageW(&msg);
            }
        }
    });

    // 2. Thread to emit events at ~60Hz
    thread::spawn(move || {
        let mut last_emit = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(16)); // ~60Hz
            let now = Instant::now();
            let dt = now.duration_since(last_emit).as_secs_f64() * 1000.0; // in ms

            if dt >= 16.0 {
                let (dx, dy) = if let Ok(mut state) = MOUSE_STATE.lock() {
                    let x = state.dx;
                    let y = state.dy;
                    state.dx = 0.0;
                    state.dy = 0.0;
                    (x, y)
                } else {
                    (0.0, 0.0)
                };
                last_emit = now;

                let speed_x = (dx.abs() / dt).max(0.0);
                let speed_y = (dy.abs() / dt).max(0.0);
                let speed_whole = ((dx * dx + dy * dy).sqrt() / dt).max(0.0);

                let _ = app_handle.emit(
                    "mouse-speed",
                    MouseSpeed {
                        speed_x,
                        speed_y,
                        speed_whole,
                    },
                );
            }
        }
    });
}
