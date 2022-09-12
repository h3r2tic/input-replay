mod hooks;
mod input;
mod play;
mod state;
mod toaster;

use std::time::Duration;

use hooks::install_and_run_hooks;
use play::play_saved_record;
use state::*;
use toaster::toast_notification;
use winrt::RuntimeContext;

fn main() {
    let rt = RuntimeContext::init();
    run();
    rt.uninit();
}

fn run() {
    std::thread::spawn(|| loop {
        handle_pending_operations(std::mem::take(&mut APP_STATE.lock().unwrap().pending));
        std::thread::sleep(Duration::from_millis(10));
    });

    install_and_run_hooks();
}

fn handle_pending_operations(pending: Vec<PendingOperation>) {
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
}
