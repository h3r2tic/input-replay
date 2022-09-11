mod toaster;

use std::{
    mem, ptr,
    sync::Mutex,
    time::{Duration, Instant},
};

use kernel32::GetModuleHandleA;
use toaster::toast_notification;
use winapi::{
    shared::{
        minwindef::*,
        ntdef::LPCSTR,
        windef::{HBRUSH, HCURSOR, HICON, HMENU, HWND, POINT},
    },
    um::winuser::{self, GetCursorPos, SetCursorPos},
};
use winrt::RuntimeContext;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum MouseButton {
    Left,
    Right,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Event {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: MouseButton, down: bool },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TimedEvent {
    event: Event,
    t_nanos: u128,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    events: Vec<TimedEvent>,
}

enum RecordingState {
    Idle,
    Recording { record: Record, start_time: Instant },
}

enum PendingOperation {
    SaveRecord { record: Record },
    PlaySavedRecord,
}

struct AppState {
    recording: RecordingState,
    pending: Vec<PendingOperation>,
}

impl AppState {
    fn mouse_action(&mut self, wparam: WPARAM, mouse_data: winuser::MSLLHOOKSTRUCT) {
        let timestamp = Instant::now();

        if let RecordingState::Recording { record, start_time } = &mut self.recording {
            let t_nanos = (timestamp.duration_since(*start_time)).as_nanos();

            if winuser::WM_LBUTTONDOWN == wparam as u32 {
                record.events.push(TimedEvent {
                    event: Event::MouseButton {
                        button: MouseButton::Left,
                        down: true,
                    },
                    t_nanos,
                });
            }

            if winuser::WM_LBUTTONUP == wparam as u32 {
                record.events.push(TimedEvent {
                    event: Event::MouseButton {
                        button: MouseButton::Left,
                        down: false,
                    },
                    t_nanos,
                });
            }

            if winuser::WM_RBUTTONDOWN == wparam as u32 {
                record.events.push(TimedEvent {
                    event: Event::MouseButton {
                        button: MouseButton::Right,
                        down: true,
                    },
                    t_nanos,
                });
            }

            if winuser::WM_RBUTTONUP == wparam as u32 {
                record.events.push(TimedEvent {
                    event: Event::MouseButton {
                        button: MouseButton::Right,
                        down: false,
                    },
                    t_nanos,
                });
            }

            if winuser::WM_MOUSEMOVE == wparam as u32 {
                let x = mouse_data.pt.x;
                let y = mouse_data.pt.y;
                record.events.push(TimedEvent {
                    event: Event::MouseMove { x, y },
                    t_nanos,
                })
            }
        }
    }

    fn key_action(&mut self, down: bool, input_key: winuser::KBDLLHOOKSTRUCT) {
        if winuser::VK_F2 == input_key.vkCode as i32 && down {
            if matches!(&self.recording, RecordingState::Idle) {
                self.recording = RecordingState::Recording {
                    record: Default::default(),
                    start_time: Instant::now(),
                };
                toast_notification("Started recording");
            } else {
                let recorded = std::mem::replace(&mut self.recording, RecordingState::Idle);
                self.recording = RecordingState::Idle;
                if let RecordingState::Recording { record, .. } = recorded {
                    let msg = format!("Stopped recording; events: {}", record.events.len());
                    toast_notification(&msg);
                    println!("{}", msg);
                    self.pending.push(PendingOperation::SaveRecord { record });
                }
            }
        }

        if winuser::VK_F3 == input_key.vkCode as i32
            && down
            && matches!(&self.recording, RecordingState::Idle)
        {
            self.pending.push(PendingOperation::PlaySavedRecord);
        }
    }
}

static APP_STATE: Mutex<AppState> = Mutex::new(AppState {
    recording: RecordingState::Idle,
    pending: vec![],
});

fn send_click(button: MouseButton, down: bool) {
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

    //unsafe { winuser::keybd_event(key, 0, if down {0} else {winuser::KEYEVENTF_KEYUP}, H3KEYS_MAGIC); }
}

pub fn play_saved_record() -> anyhow::Result<()> {
    let record = std::fs::read("record.bin")?;
    let record: Record = bincode::deserialize(&record)?;

    let t_start = Instant::now();
    for event in record.events {
        loop {
            if t_start.elapsed().as_nanos() > event.t_nanos {
                match event.event {
                    Event::MouseMove { x, y } => unsafe {
                        SetCursorPos(x, y);
                    },
                    Event::MouseButton { button, down } => {
                        send_click(button, down);
                    }
                }
                break;
            }
        }
    }

    Ok(())
}

unsafe extern "system" fn global_mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if winuser::HC_ACTION == code {
        let mouse_data = *(lparam as winuser::PMSLLHOOKSTRUCT);
        APP_STATE.lock().unwrap().mouse_action(wparam, mouse_data);
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
            APP_STATE.lock().unwrap().key_action(true, input_key);
        } else if key_up {
            APP_STATE.lock().unwrap().key_action(false, input_key);
        }
    }

    winuser::CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

fn main() {
    let rt = RuntimeContext::init();
    run();
    rt.uninit();
}

fn run() {
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

    let mut pos: POINT = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pos);
    }

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

    if 0 == unsafe { winuser::RegisterClassA(&wnd_class) } {
        panic!("RegisterClassA failed.");
    }

    let hwnd = unsafe {
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
    };

    let mut msg = winuser::MSG {
        hwnd: 0 as HWND,
        message: 0 as UINT,
        wParam: 0 as WPARAM,
        lParam: 0 as LPARAM,
        time: 0 as DWORD,
        pt: POINT { x: 0, y: 0 },
    };

    std::thread::spawn(|| loop {
        let pending = std::mem::take(&mut APP_STATE.lock().unwrap().pending);
        for op in pending {
            match op {
                PendingOperation::SaveRecord { record } => {
                    let record = bincode::serialize(&record).unwrap();
                    std::fs::write("record.bin", &record).expect("writing the record");
                }
                PendingOperation::PlaySavedRecord => {
                    toast_notification("Started playing");
                    if let Err(err) = play_saved_record() {
                        println!("Failed playing back: {:#}", err);
                        toast_notification("Failed playing");
                    } else {
                        println!("Finished playing");
                        toast_notification("Finished playing");
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    });

    loop {
        unsafe {
            if winuser::GetMessageW(&mut msg, hwnd, 0, 0) > 0 {
                winuser::TranslateMessage(&mut msg);
                winuser::DispatchMessageW(&mut msg);
            } else {
                return;
            }
        }
    }
}

pub unsafe extern "system" fn win_proc(
    h_wnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if msg == winuser::WM_DESTROY {
        winuser::PostQuitMessage(0);
    }
    return winuser::DefWindowProcW(h_wnd, msg, w_param, l_param);
}
