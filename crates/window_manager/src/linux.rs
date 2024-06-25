use x11::xlib;
use std::ptr::null_mut;
use crate::{Position, WindowError};

pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    unsafe {
        let display = xlib::XOpenDisplay(null_mut());
        if display.is_null() {
            return Err(WindowError::Other("Failed to open display".to_string()));
        }

        let root = xlib::XDefaultRootWindow(display);
        let window = find_window_by_pid(display, root, pid).ok_or(WindowError::NotFound)?;

        let mut x = 0;
        let mut y = 0;
        let mut width = 0;
        let mut height = 0;
        let mut border_width = 0;
        let mut depth = 0;
        xlib::XGetGeometry(display, window, &mut root, &mut x, &mut y, &mut width, &mut height, &mut border_width, &mut depth);

        xlib::XCloseDisplay(display);

        Ok(Position {
            x,
            y,
            width,
            height,
        })
    }
}

pub fn set_window_position(pid: u32, pos: Position) -> Result<(), WindowError> {
    unsafe {
        let display = xlib::XOpenDisplay(null_mut());
        if display.is_null() {
            return Err(WindowError::Other("Failed to open display".to_string()));
        }

        let root = xlib::XDefaultRootWindow(display);
        let window = find_window_by_pid(display, root, pid).ok_or(WindowError::NotFound)?;

        xlib::XMoveResizeWindow(display, window, pos.x, pos.y, pos.width, pos.height);

        xlib::XCloseDisplay(display);

        Ok(())
    }
}

fn find_window_by_pid(display: *mut xlib::Display, root: xlib::Window, pid: u32) -> Option<xlib::Window> {
    unsafe {
        let mut window = root;
        let mut root_return = 0;
        let mut parent_return = 0;
        let mut children_return: *mut xlib::Window = null_mut();
        let mut nchildren_return = 0;

        xlib::XQueryTree(display, root, &mut root_return, &mut parent_return, &mut children_return, &mut nchildren_return);
        let children = std::slice::from_raw_parts(children_return, nchildren_return as usize);

        for &child in children {
            let mut actual_type_return = 0;
            let mut actual_format_return = 0;
            let mut nitems_return = 0;
            let mut bytes_after_return = 0;
            let mut prop_return: *mut u64 = null_mut();

            xlib::XGetWindowProperty(
                display,
                child,
                xlib::XA_WM_CLASS,
                0,
                std::mem::size_of::<u64>() as i64,
                0,
                xlib::XA_WM_CLASS,
                &mut actual_type_return,
                &mut actual_format_return,
                &mut nitems_return,
                &mut bytes_after_return,
                &mut prop_return as *mut *mut u64 as *mut *mut u8,
            );

            if !prop_return.is_null() && *prop_return == pid as u64 {
                xlib::XFree(prop_return as *mut _);
                return Some(child);
            }
            xlib::XFree(prop_return as *mut _);
        }
        None
    }
}