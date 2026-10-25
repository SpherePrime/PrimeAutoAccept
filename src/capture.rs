use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, COLORONCOLOR, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, HGDIOBJ, ReleaseDC,
    SRCCOPY, SelectObject, SetStretchBltMode, StretchBlt,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};

use crate::detector::{Frame, Point, SEARCH_AREA};
use crate::window::GameWindow;

const PW_CLIENT_FULL_CONTENT: u32 = 3;
const CAPTURE_WIDTH: i32 = 320;

pub struct Capture {
    pub frame: Frame,
    origin: Point,
    scale: f32,
}

impl Capture {
    pub fn to_screen(&self, point: Point) -> Point {
        Point {
            x: self.origin.x + (point.x as f32 * self.scale) as i32,
            y: self.origin.y + (point.y as f32 * self.scale) as i32,
        }
    }
}

struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

pub fn search_area(window: GameWindow) -> Option<Capture> {
    if window.is_minimized() {
        return None;
    }
    let client = window.client_area()?;
    if client.width < 200 || client.height < 200 {
        return None;
    }
    let area = Rect {
        x: (client.width as f32 * SEARCH_AREA.left) as i32,
        y: (client.height as f32 * SEARCH_AREA.top) as i32,
        width: (client.width as f32 * (SEARCH_AREA.right - SEARCH_AREA.left)) as i32,
        height: (client.height as f32 * (SEARCH_AREA.bottom - SEARCH_AREA.top)) as i32,
    };
    let out_width = area.width.min(CAPTURE_WIDTH);
    let out_height = (area.height as i64 * out_width as i64 / area.width as i64) as i32;
    let frame = if window.is_foreground() {
        from_screen(&client.origin, &area, out_width, out_height)
    } else {
        from_window(window.hwnd(), client.width, client.height, &area, out_width, out_height)
    }?;
    Some(Capture {
        frame,
        origin: Point {
            x: client.origin.x + area.x,
            y: client.origin.y + area.y,
        },
        scale: area.width as f32 / out_width as f32,
    })
}

fn from_screen(origin: &Point, area: &Rect, out_width: i32, out_height: i32) -> Option<Frame> {
    let screen = DeviceContext::of(None)?;
    let canvas = Canvas::new(screen.dc, out_width, out_height)?;
    let copied = canvas.shrink_from(screen.dc, origin.x + area.x, origin.y + area.y, area);
    canvas.into_frame(copied)
}

fn from_window(
    hwnd: HWND,
    client_width: i32,
    client_height: i32,
    area: &Rect,
    out_width: i32,
    out_height: i32,
) -> Option<Frame> {
    let source = DeviceContext::of(Some(hwnd))?;
    let full = Canvas::new(source.dc, client_width, client_height)?;
    let printed = unsafe { PrintWindow(hwnd, full.dc, PRINT_WINDOW_FLAGS(PW_CLIENT_FULL_CONTENT)) }
        .as_bool();
    if !printed {
        return None;
    }
    let canvas = Canvas::new(source.dc, out_width, out_height)?;
    let copied = canvas.shrink_from(full.dc, area.x, area.y, area);
    canvas
        .into_frame(copied)
        .filter(|frame| frame.bgra.chunks_exact(4).any(|px| px[0] | px[1] | px[2] != 0))
}

struct DeviceContext {
    window: HWND,
    dc: HDC,
}

impl DeviceContext {
    fn of(window: Option<HWND>) -> Option<DeviceContext> {
        let window = window.unwrap_or_default();
        let dc = unsafe { GetDC(window) };
        (!dc.is_invalid()).then_some(DeviceContext { window, dc })
    }
}

impl Drop for DeviceContext {
    fn drop(&mut self) {
        unsafe { ReleaseDC(self.window, self.dc) };
    }
}

struct Canvas {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: i32,
    height: i32,
}

impl Canvas {
    fn new(compatible: HDC, width: i32, height: i32) -> Option<Canvas> {
        unsafe {
            let dc = CreateCompatibleDC(compatible);
            if dc.is_invalid() {
                return None;
            }
            let bitmap = CreateCompatibleBitmap(compatible, width, height);
            if bitmap.is_invalid() {
                let _ = DeleteDC(dc);
                return None;
            }
            let previous = SelectObject(dc, bitmap);
            Some(Canvas {
                dc,
                bitmap,
                previous,
                width,
                height,
            })
        }
    }

    fn shrink_from(&self, source: HDC, x: i32, y: i32, area: &Rect) -> bool {
        unsafe {
            SetStretchBltMode(self.dc, COLORONCOLOR);
            StretchBlt(
                self.dc,
                0,
                0,
                self.width,
                self.height,
                source,
                x,
                y,
                area.width,
                area.height,
                SRCCOPY,
            )
            .as_bool()
        }
    }

    fn into_frame(self, drawn: bool) -> Option<Frame> {
        if !drawn {
            return None;
        }
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: self.width,
                biHeight: -self.height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; self.width as usize * self.height as usize * 4];
        let lines = unsafe {
            SelectObject(self.dc, self.previous);
            GetDIBits(
                self.dc,
                self.bitmap,
                0,
                self.height as u32,
                Some(bgra.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        (lines == self.height).then(|| Frame {
            width: self.width as usize,
            height: self.height as usize,
            bgra,
        })
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.dc);
        }
    }
}

