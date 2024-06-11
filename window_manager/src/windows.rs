use std::mem;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowThreadProcessId, SetWindowPos, SWP_NOZORDER,
};
use crate::{Position, WindowError};
use std::ptr::{null, null_mut};

pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    let hwnd = find_window_by_pid(pid).ok_or(WindowError::NotFound)?;
    let rect = get_window_rect(hwnd)?;
    Ok(Position {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left) as u32,
        height: (rect.bottom - rect.top) as u32,
    })
}

pub fn set_window_position(pid: u32, pos: Position) -> Result<(), WindowError> {
    let hwnd = find_window_by_pid(pid).ok_or(WindowError::NotFound)?;
    unsafe {
        if SetWindowPos(
            hwnd,
            0,
            pos.x,
            pos.y,
            pos.width as i32,
            pos.height as i32,
            SWP_NOZORDER,
        ) != 0
        {
            Ok(())
        } else {
            Err(WindowError::Other("Failed to set window position".to_string()))
        }
    }
}

fn find_window_by_pid(pid: u32) -> Option<HWND> {
    let mut data: (Option<HWND>, u32) = (None, pid);
    unsafe {
        EnumWindows(Some(enum_windows_proc), &mut data as *mut _ as isize);
    }

    data.0
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: isize) -> i32 {
    let mut process_id = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);


    let data: &mut (Option<HWND>, u32) = mem::transmute::<_,_>(lparam);

    if process_id == data.1 {
        data.0 = Some(hwnd);
        return 0; // Stop enumeration
    }
    1 // Continue enumeration
}

fn get_window_rect(hwnd: HWND) -> Result<RECT, WindowError> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        if GetWindowRect(hwnd, &mut rect) != 0 {
            Ok(rect)
        } else {
            Err(WindowError::Other("Failed to get window rect".to_string()))
        }
    }
}