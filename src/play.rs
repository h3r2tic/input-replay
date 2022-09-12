use std::time::Instant;

use crate::input::{send_click, send_cursor_pos};
use crate::state::*;
use winapi::um::winuser;

pub fn play_saved_record() -> anyhow::Result<()> {
    let record = std::fs::read("record.bin")?;
    let record: Record = bincode::deserialize(&record)?;

    let screen_dims = unsafe {
        [
            winuser::GetSystemMetrics(winuser::SM_CXSCREEN),
            winuser::GetSystemMetrics(winuser::SM_CYSCREEN),
        ]
    };

    let t_start = Instant::now();
    for event in record.events {
        loop {
            if t_start.elapsed().as_nanos() > event.t_nanos {
                match event.event {
                    Event::MouseMove { x, y } => {
                        send_cursor_pos(x, y, screen_dims);
                    }
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
