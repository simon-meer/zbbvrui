use core_graphics::display::{CGGetActiveDisplayList, CGDisplayBounds};
use core_graphics::window::{kCGWindowListOptionOnScreenOnly, kCGNullWindowID, CGWindowListCopyWindowInfo, CGWindowID};
use std::collections::HashMap;
use crate::{Position, WindowError};

pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    let window_info = find_window_by_pid(pid).ok_or(WindowError::NotFound)?;
    let position = get_window_rect(&window_info)?;
    Ok(position)
}

pub fn set_window_position(_pid: u32, _pos: Position) -> Result<(), WindowError> {
    // macOS window position manipulation is non-trivial and usually handled by AppleScript or other higher-level mechanisms.
    Err(WindowError::Other("Setting window position is not implemented on macOS".to_string()))
}

fn find_window_by_pid(pid: u32) -> Option<HashMap<String, core_graphics::base::CFType>> {
    let window_list_info = unsafe {
        CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)
    };
    let windows: Vec<HashMap<String, core_graphics::base::CFType>> = core_graphics::window::CGWindowListOption::from_window_list_info(window_list_info);

    for window in windows {
        let window_pid: i64 = window.get("kCGWindowOwnerPID")?.as_i64()?;
        if window_pid == pid as i64 {
            return Some(window);
        }
    }
    None
}

fn get_window_rect(window: &HashMap<String, core_graphics::base::CFType>) -> Result<Position, WindowError> {
    let bounds = window.get("kCGWindowBounds").ok_or(WindowError::Other("No bounds found".to_string()))?.as_dict().ok_or(WindowError::Other("Invalid bounds".to_string()))?;
    let x = bounds.get("X").ok_or(WindowError::Other("No X found".to_string()))?.as_f64().ok_or(WindowError::Other("Invalid X".to_string()))?;
    let y = bounds.get("Y").ok_or(WindowError::Other("No Y found".to_string()))?.as_f64().ok_or(WindowError::Other("Invalid Y".to_string()))?;
    let width = bounds.get("Width").ok_or(WindowError::Other("No Width found".to_string()))?.as_f64().ok_or(WindowError::Other("Invalid Width".to_string()))?;
    let height = bounds.get("Height").ok_or(WindowError::Other("No Height found".to_string()))?.as_f64().ok_or(WindowError::Other("Invalid Height".to_string()))?;
    Ok(Position {
        x: x as i32,
        y: y as i32,
        width: width as u32,
        height: height as u32,
    })
}