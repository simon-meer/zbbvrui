use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowError {
    NotFound,
    Other(String),
}

#[cfg(target_os = "windows")]
pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    windows::get_window_position(pid)
}

#[cfg(target_os = "windows")]
pub fn set_window_position(pid: u32, pos: Position) -> Result<(), WindowError> {
    windows::set_window_position(pid, pos)
}

#[cfg(target_os = "linux")]
pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    linux::get_window_position(pid)
}

#[cfg(target_os = "linux")]
pub fn set_window_position(pid: u32, pos: Position) -> Result<(), WindowError> {
    linux::set_window_position(pid, pos)
}

#[cfg(target_os = "macos")]
pub fn get_window_position(pid: u32) -> Result<Position, WindowError> {
    macos::get_window_position(pid)
}

#[cfg(target_os = "macos")]
pub fn set_window_position(pid: u32, pos: Position) -> Result<(), WindowError> {
    macos::set_window_position(pid, pos)
}