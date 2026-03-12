use std::thread;
use std::time::Duration;

use rdev::{simulate, EventType, Key};

pub fn send_paste_shortcut_with_retry(
    max_attempts: u8,
    retry_delay: Duration,
) -> Result<(), String> {
    let attempts = max_attempts.max(1);
    let mut last_error: Option<String> = None;

    for index in 0..attempts {
        match send_paste_shortcut_once() {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                if index + 1 < attempts {
                    thread::sleep(retry_delay);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "发送粘贴快捷键失败".to_string()))
}

fn send_paste_shortcut_once() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let modifier = Key::MetaLeft;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::ControlLeft;

    simulate_event(EventType::KeyPress(modifier))?;
    simulate_event(EventType::KeyPress(Key::KeyV))?;
    simulate_event(EventType::KeyRelease(Key::KeyV))?;
    simulate_event(EventType::KeyRelease(modifier))
}

fn simulate_event(event: EventType) -> Result<(), String> {
    simulate(&event).map_err(|error| format!("自动粘贴失败，请切回目标窗口后重试: {error}"))?;
    thread::sleep(Duration::from_millis(7));
    Ok(())
}
