use std::mem;
use winapi::um::winuser::{self};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
}

pub fn send_click(button: MouseButton, down: bool) {
    unsafe {
        let mut input = winuser::INPUT {
            type_: winuser::INPUT_MOUSE,
            u: Default::default(),
        };

        *input.u.mi_mut() = winuser::MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: match (button, down) {
                (MouseButton::Left, true) => winuser::MOUSEEVENTF_LEFTDOWN,
                (MouseButton::Left, false) => winuser::MOUSEEVENTF_LEFTUP,
                (MouseButton::Right, true) => winuser::MOUSEEVENTF_RIGHTDOWN,
                (MouseButton::Right, false) => winuser::MOUSEEVENTF_RIGHTUP,
            },
            time: 0,
            dwExtraInfo: 0,
        };

        winuser::SendInput(1, &mut input, mem::size_of::<winuser::INPUT>() as i32);
    }
}

pub fn send_cursor_pos(x: i32, y: i32, screen_dims: [i32; 2]) {
    unsafe {
        let mut input = winuser::INPUT {
            type_: winuser::INPUT_MOUSE,
            u: Default::default(),
        };

        *input.u.mi_mut() = winuser::MOUSEINPUT {
            dx: ((x as i64) * 65535 / screen_dims[0] as i64) as i32,
            dy: ((y as i64) * 65535 / screen_dims[1] as i64) as i32,
            mouseData: 0,
            dwFlags: winuser::MOUSEEVENTF_MOVE
                | winuser::MOUSEEVENTF_MOVE_NOCOALESCE
                | winuser::MOUSEEVENTF_ABSOLUTE,
            time: 0,
            dwExtraInfo: 0,
        };

        winuser::SendInput(1, &mut input, mem::size_of::<winuser::INPUT>() as i32);
    }
}
