use std::time::Duration;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SRCCOPY, SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetClientRect, GetCursorPos, GetForegroundWindow, GetWindow,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SetCursorPos,
    SetForegroundWindow, ShowWindow,
};

use crate::detector::{Frame, Point};

const PW_CLIENT_FULL_CONTENT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameWindow(isize);

pub struct Capture {
    pub frame: Frame,
    pub origin: Point,
}

pub fn enable_dpi_awareness() {
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
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

    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }

    pub fn is_foreground(self) -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        foreground == self.hwnd()
    }

    pub fn is_minimized(self) -> bool {
        unsafe { IsIconic(self.hwnd()) }.as_bool()
    }

    pub fn capture(self) -> Option<Capture> {
        if self.is_minimized() {
            return None;
        }
        let hwnd = self.hwnd();
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
        let width = (rect.right - rect.left).max(0);
        let height = (rect.bottom - rect.top).max(0);
        if width < 64 || height < 64 {
            return None;
        }
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
            return None;
        }
        let pixels = if self.is_foreground() {
            grab_screen(origin, width, height)
        } else {
            grab_window(hwnd, width, height)
        }?;
        Some(Capture {
            frame: Frame {
                width: width as usize,
                height: height as usize,
                bgra: pixels,
            },
            origin: Point {
                x: origin.x,
                y: origin.y,
            },
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
        send(&[key_input(KEYBD_EVENT_FLAGS(0)), key_input(KEYEVENTF_KEYUP)]);
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}

pub fn click_at(target: Point) {
    let mut previous = POINT::default();
    let had_cursor = unsafe { GetCursorPos(&mut previous) }.is_ok();
    if unsafe { SetCursorPos(target.x, target.y) }.is_err() {
        return;
    }
    std::thread::sleep(Duration::from_millis(60));
    send(&[mouse_input(MOUSEEVENTF_LEFTDOWN)]);
    std::thread::sleep(Duration::from_millis(50));
    send(&[mouse_input(MOUSEEVENTF_LEFTUP)]);
    std::thread::sleep(Duration::from_millis(60));
    if had_cursor {
        let _ = unsafe { SetCursorPos(previous.x, previous.y) };
    }
}

fn grab_screen(origin: POINT, width: i32, height: i32) -> Option<Vec<u8>> {
    with_bitmap(None, width, height, |memory_dc, screen_dc| unsafe {
        BitBlt(memory_dc, 0, 0, width, height, screen_dc, origin.x, origin.y, SRCCOPY).is_ok()
    })
}

fn grab_window(hwnd: HWND, width: i32, height: i32) -> Option<Vec<u8>> {
    with_bitmap(Some(hwnd), width, height, |memory_dc, _| unsafe {
        PrintWindow(hwnd, memory_dc, PRINT_WINDOW_FLAGS(PW_CLIENT_FULL_CONTENT)).as_bool()
    })
    .filter(|pixels| pixels.chunks_exact(4).any(|px| px[0] | px[1] | px[2] != 0))
}

fn with_bitmap(
    source: Option<HWND>,
    width: i32,
    height: i32,
    draw: impl FnOnce(
        windows::Win32::Graphics::Gdi::HDC,
        windows::Win32::Graphics::Gdi::HDC,
    ) -> bool,
) -> Option<Vec<u8>> {
    unsafe {
        let source_dc = GetDC(source.unwrap_or_default());
        if source_dc.is_invalid() {
            return None;
        }
        let memory_dc = CreateCompatibleDC(source_dc);
        let bitmap = CreateCompatibleBitmap(source_dc, width, height);
        let previous = SelectObject(memory_dc, bitmap);
        let drawn = draw(memory_dc, source_dc);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        SelectObject(memory_dc, previous);
        let lines = if drawn {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(memory_dc);
        ReleaseDC(source.unwrap_or_default(), source_dc);
        (lines == height).then_some(pixels)
    }
}

fn send(inputs: &[INPUT]) {
    unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn key_input(flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_MENU,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}
