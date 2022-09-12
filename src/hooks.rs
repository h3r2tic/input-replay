use kernel32::GetModuleHandleA;
use std::ptr;
use winapi::{
    shared::{
        minwindef::*,
        ntdef::LPCSTR,
        windef::{HBRUSH, HCURSOR, HICON, HMENU, HWND},
    },
    um::winuser::{self},
};

use crate::state::*;

pub fn install_and_run_hooks() {
    hook_keyboard_and_mouse();
    let hwnd = create_dummy_window();
    run_window_message_pump(hwnd);
}

fn run_window_message_pump(hwnd: HWND) {
    let mut msg = winuser::MSG::default();
    loop {
        unsafe {
            if winuser::GetMessageW(&mut msg, hwnd, 0, 0) > 0 {
                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            } else {
                return;
            }
        }
    }
}

fn create_dummy_window() -> HWND {
    let class_name = "input-replay";
    let wnd_class = winuser::WNDCLASSA {
        style: 0,
        lpfnWndProc: Some(win_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: 0 as HINSTANCE,
        hIcon: 0 as HICON,
        hCursor: 0 as HCURSOR,
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: 0 as LPCSTR,
        lpszClassName: class_name.as_ptr() as *const i8,
    };

    assert!(
        unsafe { winuser::RegisterClassA(&wnd_class) } != 0,
        "RegisterClassA failed."
    );

    unsafe {
        winuser::CreateWindowExA(
            0,
            class_name.as_ptr() as *const i8,
            class_name.as_ptr() as *const i8,
            0,
            winuser::CW_USEDEFAULT,
            winuser::CW_USEDEFAULT,
            320,
            240,
            winuser::GetDesktopWindow(),
            0 as HMENU,
            0 as HINSTANCE,
            std::ptr::null_mut(),
        )
    }
}

fn hook_keyboard_and_mouse() {
    unsafe {
        winuser::SetWindowsHookExA(
            winuser::WH_KEYBOARD_LL,
            Some(global_key_hook),
            GetModuleHandleA(ptr::null()) as HINSTANCE,
            0,
        );
        winuser::SetWindowsHookExA(
            winuser::WH_MOUSE_LL,
            Some(global_mouse_hook),
            GetModuleHandleA(ptr::null()) as HINSTANCE,
            0,
        );
    }
}

/// # Safety
///
/// No null pointers plz
pub unsafe extern "system" fn win_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if msg == winuser::WM_DESTROY {
        winuser::PostQuitMessage(0);
    }

    winuser::DefWindowProcW(hwnd, msg, w_param, l_param)
}

unsafe extern "system" fn global_mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if winuser::HC_ACTION == code {
        let mouse_data = *(lparam as winuser::PMSLLHOOKSTRUCT);
        APP_STATE
            .lock()
            .unwrap()
            .on_mouse_action(wparam, mouse_data);
    }

    winuser::CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn global_key_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if winuser::HC_ACTION == code {
        let input_key = *(lparam as winuser::PKBDLLHOOKSTRUCT);

        let key_down =
            winuser::WM_KEYDOWN == wparam as u32 || winuser::WM_SYSKEYDOWN == wparam as u32;
        let key_up = winuser::WM_KEYUP == wparam as u32 || winuser::WM_SYSKEYUP == wparam as u32;

        if key_down {
            APP_STATE.lock().unwrap().on_key_action(true, input_key);
        } else if key_up {
            APP_STATE.lock().unwrap().on_key_action(false, input_key);
        }
    }

    winuser::CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}
