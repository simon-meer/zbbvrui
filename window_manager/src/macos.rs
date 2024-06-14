use core_foundation::base::TCFType;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::{CFNumber, CFNumberRef};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::display::{CFArrayGetCount, CFArrayGetValueAtIndex, CFDictionaryGetValueIfPresent, CFDictionaryRef, CGRect};
use core_graphics::window::{CGWindowListCopyWindowInfo, kCGNullWindowID, kCGWindowBounds, kCGWindowListExcludeDesktopElements, kCGWindowOwnerName, kCGWindowOwnerPID};

use crate::{Position, WindowError};

pub fn get_window_position(pid: i32) -> Result<Position, WindowError> {
    find_window_by_pid(pid).map_err(|_| WindowError::NotFound)
}

pub fn set_window_position(_pid: u32, _pos: Position) -> Result<(), WindowError> {
    // macOS window position manipulation is non-trivial and usually handled by AppleScript or other higher-level mechanisms.
    Err(WindowError::Other("Setting window position is not implemented on macOS".to_string()))
}

pub fn find_window_by_pid(pid: i32) -> Result<Position, String> {
    use std::ffi::c_void;

    unsafe {
        let cf_win_array =
            CGWindowListCopyWindowInfo(kCGWindowListExcludeDesktopElements, kCGNullWindowID);
        let count = CFArrayGetCount(cf_win_array);

        if count == 0 {
            return Err("No game window found".to_string());
        }

        let mut mrect = Position::default();
        let mut window_count = 0;
        let mut title: String = String::new();

        for i in 0..count {
            let win_info_ref: CFDictionaryRef =
                CFArrayGetValueAtIndex(cf_win_array, i) as CFDictionaryRef;
            let mut test_pid_ref: *const c_void = std::ptr::null_mut();
            assert_ne!(CFDictionaryGetValueIfPresent(
                win_info_ref,
                kCGWindowOwnerPID as *const c_void,
                &mut test_pid_ref,
            ), 0);
            let test_pid = CFNumber::wrap_under_get_rule(test_pid_ref as CFNumberRef);

            if pid == test_pid.to_i32().unwrap() {
                let mut cg_bounds_dict_ref: *const c_void = std::ptr::null_mut();
                CFDictionaryGetValueIfPresent(
                    win_info_ref,
                    kCGWindowBounds as *const c_void,
                    &mut cg_bounds_dict_ref,
                );
                let cg_bounds_dict =
                    CFDictionary::wrap_under_get_rule(cg_bounds_dict_ref as CFDictionaryRef);
                let cg_rect = CGRect::from_dict_representation(&cg_bounds_dict).unwrap();

                let mut cg_title_ref: *const c_void = std::ptr::null_mut();
                CFDictionaryGetValueIfPresent(
                    win_info_ref,
                    kCGWindowOwnerName as *const c_void,
                    &mut cg_title_ref,
                );
                let cg_title = CFString::wrap_under_get_rule(cg_title_ref as CFStringRef);
                title = cg_title.to_string();
                if cg_rect.size.height > 200. {
                    mrect = Position {
                        x: cg_rect.origin.x.round() as i32,
                        y: cg_rect.origin.y.round() as i32,
                        width: cg_rect.size.width.round() as u32,
                        height: cg_rect.size.height.round() as u32,
                    };

                    window_count += 1
                }
            }
        }
        if window_count > 0 {
            Ok(mrect)
        } else {
            Err("No genshin window found".to_string())
        }
    }
}