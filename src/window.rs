use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{
    BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess, SetPriorityClass,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetClientRect, GetForegroundWindow, GetWindow,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow,
    ShowWindow,
};

use crate::detector::Point;
use crate::input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameWindow(isize);

pub struct ClientArea {
    pub origin: Point,
    pub width: i32,
    pub height: i32,
}

pub fn prepare_process() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
}

struct Search {
    pid: u32,
    best: Option<(HWND, i64)>,
}

unsafe extern "system" fn visit_window(hwnd: HWND, data: LPARAM) -> BOOL {
    let search = unsafe { &mut *(data.0 as *mut Search) };
    let mut owner_pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) };
    let is_top_level = unsafe { GetWindow(hwnd, GW_OWNER) }.is_err();
    if owner_pid == search.pid && is_top_level && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) }.is_ok() {
            let area = (rect.right - rect.left) as i64 * (rect.bottom - rect.top) as i64;
            if search.best.is_none_or(|(_, best)| area > best) {
                search.best = Some((hwnd, area));
            }
        }
    }
    BOOL(1)
}

impl GameWindow {
    pub fn of_process(pid: u32) -> Option<GameWindow> {
        let mut search = Search { pid, best: None };
        let _ = unsafe { EnumWindows(Some(visit_window), LPARAM(&mut search as *mut _ as isize)) };
        search.best.map(|(hwnd, _)| GameWindow(hwnd.0 as isize))
    }

    pub fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }

    pub fn is_foreground(self) -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        foreground == self.hwnd()
    }

    pub fn is_minimized(self) -> bool {
        unsafe { IsIconic(self.hwnd()) }.as_bool()
    }

    pub fn client_area(self) -> Option<ClientArea> {
        let hwnd = self.hwnd();
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
            return None;
        }
        Some(ClientArea {
            origin: Point {
                x: origin.x,
                y: origin.y,
            },
            width: (rect.right - rect.left).max(0),
            height: (rect.bottom - rect.top).max(0),
        })
    }

    pub fn bring_to_front(self) {
        let hwnd = self.hwnd();
        if self.is_minimized() {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
        }
        if self.is_foreground() {
            return;
        }
        input::tap_alt();
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}
