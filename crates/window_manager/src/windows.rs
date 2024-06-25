use std::mem;
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClientRect, GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, SetWindowPos, SW_SHOWMINIMIZED, SWP_NOZORDER, WINDOWPLACEMENT};
use crate::{Position, WindowError};

const DEFAULT_WINDOWPLACEMENT: WINDOWPLACEMENT = WINDOWPLACEMENT {
    length: 0,
    flags: 0,
    showCmd: 0,
    ptMinPosition: POINT { x: 0, y: 0},
    ptMaxPosition: POINT { x: 0, y: 0},
    rcNormalPosition: RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    },
};

const DEFAULT_RECT: RECT = RECT  {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};

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
    let mut win_rect = DEFAULT_RECT.clone();
    let mut client_rect = DEFAULT_RECT.clone();
    let mut window_placement = DEFAULT_WINDOWPLACEMENT.clone();

    unsafe {
        if GetWindowPlacement(hwnd, &mut window_placement) == 0 {
            return Err(WindowError::Other("Failed to get window placement".to_string()));
        }
        
        if window_placement.showCmd == SW_SHOWMINIMIZED as u32 {
            return Err(WindowError::Other("Window is minimized".to_string()));
        }
        
        if GetClientRect(hwnd, &mut client_rect) != 0 && GetWindowRect(hwnd, &mut win_rect) != 0 {
            let border_width = ((win_rect.right - win_rect.left) - client_rect.right) / 2;
            let header_height = ((win_rect.bottom - win_rect.top) - client_rect.bottom) - border_width;
            
            Ok(RECT {
                left: win_rect.left + border_width,
                right: win_rect.right - border_width,
                top: win_rect.top + header_height,
                bottom: win_rect.bottom - border_width
            })
        } else {
            Err(WindowError::Other("Failed to get window rect".to_string()))
        }
    }
}