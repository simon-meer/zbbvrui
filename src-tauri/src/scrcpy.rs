use std::process::Command;
use tauri::AppHandle;
use crate::structs::ZBBError;

pub struct ScrCpy {
}

impl ScrCpy {
    pub fn open_window(id: &str, handle: &AppHandle) -> Result<(), ZBBError> {
        let scrcpy = handle.path_resolver()
            .resolve_resource("scrcpy/scrcpy.exe")
            .ok_or(ZBBError::Other("Scrcpy nicht gefunden".to_string()))?;

        Command::new(scrcpy).args(vec![
           "-s", id
        ]).output()?;

        Ok(())
    }
}