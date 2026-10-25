use std::time::Duration;

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

use crate::detector::Point;

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

pub fn tap_alt() {
    send(&[key_input(KEYBD_EVENT_FLAGS(0)), key_input(KEYEVENTF_KEYUP)]);
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
