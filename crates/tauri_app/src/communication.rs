use async_std::io::{ReadExt, WriteExt};

use crate::structs::{AppPhase, ZBBError};

const SOCKET_PORT: u16 = 1337;

#[tauri::command]
pub async fn get_phase(ip: String) -> Result<AppPhase, ZBBError> {
    let ip_address = format!("{}:{}", ip, SOCKET_PORT);
    let mut socket = async_std::net::TcpStream::connect(ip_address).await?;

    let mut buffer = [0u8; 128];
    socket.write(b"get_phase").await?;

    let size = socket.read(&mut buffer).await?;
    let response = String::from_utf8(buffer[0..size].to_vec())?;

    response
        .parse::<AppPhase>()
        .map_err(|_| ZBBError::IO("Konnte Status nicht lesen.".into()))
}

#[tauri::command]
pub async fn set_phase(ip: String, phase: AppPhase) -> Result<(), ZBBError> {
    let ip_address = format!("{}:{}", ip, SOCKET_PORT);
    let mut socket = async_std::net::TcpStream::connect(ip_address).await?;

    let payload = format!("set_phase {}", phase);
    socket.write(payload.as_bytes()).await?;

    let mut buffer = [0u8; 128];
    let size = socket.read(&mut buffer).await?;
    let response = String::from_utf8(buffer[0..size].to_vec())?;

    if response == "ok" {
        Ok(())
    } else {
        Err(ZBBError::Other(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_set_phase() {
        let result = set_phase("127.0.0.1".to_string(), AppPhase::Windup).await;

        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_phase() {
        let result = get_phase("127.0.0.1".to_string()).await;

        println!("{:?}", result);
        assert_eq!(result.unwrap(), AppPhase::Onboarding);
    }
}
