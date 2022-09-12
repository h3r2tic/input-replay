use crate::{input::MouseButton, toaster::toast_notification};
use std::{sync::Mutex, time::Instant};
use winapi::{
    shared::{minwindef::*, windef::POINT},
    um::winuser::{self, GetCursorPos},
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Event {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: MouseButton, down: bool },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TimedEvent {
    pub event: Event,
    pub t_nanos: u128,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub events: Vec<TimedEvent>,
}

pub enum RecordingState {
    Idle,
    Recording { record: Record, start_time: Instant },
}

pub enum PendingOperation {
    SaveRecord { record: Record },
    PlaySavedRecord,
}

pub struct AppState {
    pub recording: RecordingState,
    pub pending: Vec<PendingOperation>,
    pub modkey_down: bool,
}

impl AppState {
    pub fn on_mouse_action(&mut self, wparam: WPARAM, mouse_data: winuser::MSLLHOOKSTRUCT) {
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

    pub fn on_key_action(&mut self, down: bool, input_key: winuser::KBDLLHOOKSTRUCT) {
        if winuser::VK_LWIN == input_key.vkCode as i32 {
            self.modkey_down = down;
        }

        if self.modkey_down {
            if winuser::VK_F2 == input_key.vkCode as i32 && down {
                if matches!(&self.recording, RecordingState::Idle) {
                    let mut cursor_pos: POINT = POINT { x: 0, y: 0 };
                    unsafe {
                        GetCursorPos(&mut cursor_pos);
                    }

                    self.recording = RecordingState::Recording {
                        record: Record {
                            events: vec![TimedEvent {
                                event: Event::MouseMove {
                                    x: cursor_pos.x,
                                    y: cursor_pos.y,
                                },
                                t_nanos: 0,
                            }],
                        },
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
}

pub static APP_STATE: Mutex<AppState> = Mutex::new(AppState {
    recording: RecordingState::Idle,
    pending: vec![],
    modkey_down: false,
});
