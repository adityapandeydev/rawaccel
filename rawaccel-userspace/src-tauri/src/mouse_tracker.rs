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
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, RegisterClassW,
    HWND_MESSAGE, MSG, WNDCLASSW,
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
    distance: f64,
}

impl MouseState {
    fn record(&mut self, x: i32, y: i32) {
        let x = x as f64;
        let y = y as f64;
        self.dx += x.abs();
        self.dy += y.abs();
        self.distance += x.hypot(y);
    }

    fn take(&mut self, milliseconds: f64) -> MouseSpeed {
        let divisor = if milliseconds.is_finite() && milliseconds > 0.0 {
            milliseconds
        } else {
            f64::INFINITY
        };
        let result = MouseSpeed {
            speed_x: self.dx / divisor,
            speed_y: self.dy / divisor,
            speed_whole: self.distance / divisor,
        };
        self.dx = 0.0;
        self.dy = 0.0;
        self.distance = 0.0;
        result
    }
}

// Distance traveled across all relative mouse packets, not net displacement.
// WM_INPUT does not establish whether counts are before or after the filter.
static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState {
    dx: 0.0,
    dy: 0.0,
    distance: 0.0,
});

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
                state.record(x, y);
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
                let _ = DestroyWindow(hwnd);
                return;
            }

            // Run message loop
            let mut msg = MSG::default();
            loop {
                match GetMessageW(&mut msg, None, 0, 0).0 {
                    -1 => {
                        eprintln!(
                            "Mouse tracker message loop failed: {}",
                            windows::core::Error::from_thread()
                        );
                        break;
                    }
                    0 => break, // WM_QUIT
                    _ => {
                        DispatchMessageW(&msg);
                    }
                }
            }
            let _ = DestroyWindow(hwnd);
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
                let speed = if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.take(dt)
                } else {
                    MouseSpeed {
                        speed_x: 0.0,
                        speed_y: 0.0,
                        speed_whole: 0.0,
                    }
                };
                last_emit = now;

                let _ = app_handle.emit("mouse-speed", speed);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> MouseState {
        MouseState {
            dx: 0.0,
            dy: 0.0,
            distance: 0.0,
        }
    }

    #[test]
    fn reversals_preserve_path_distance_and_window_resets() {
        let mut state = empty();
        state.record(3, 4);
        state.record(-3, -4);
        let speed = state.take(2.0);
        assert_eq!(
            (speed.speed_x, speed.speed_y, speed.speed_whole),
            (3.0, 4.0, 5.0)
        );
        assert_eq!(state.take(16.0).speed_whole, 0.0);
    }

    #[test]
    fn orthogonal_packets_sum_lengths_instead_of_combining_displacements() {
        let mut state = empty();
        state.record(3, 0);
        state.record(0, 4);
        assert_eq!(state.take(1.0).speed_whole, 7.0);
    }

    #[test]
    fn invalid_windows_and_extreme_relative_counts_remain_finite() {
        for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut state = empty();
            state.record(i32::MIN, i32::MAX);
            assert_eq!(state.take(duration).speed_whole, 0.0);
            assert_eq!(state.take(16.0).speed_x, 0.0);
        }
        let mut state = empty();
        state.record(i32::MIN, i32::MAX);
        assert!(state.take(0.125).speed_whole.is_finite());
    }
}
