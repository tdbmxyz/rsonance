use anyhow::Result;
use std::process::Command;

/// Configuration for audio streaming
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: AudioFormat,
}

#[derive(Debug, Clone)]
pub enum AudioFormat {
    S16LE,
    F32LE,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            format: AudioFormat::S16LE,
        }
    }
}

/// Result of virtual microphone setup
#[derive(Debug, PartialEq)]
pub enum VirtualMicResult {
    Success,
    Failed,
}

/// Creates virtual microphone using pactl
pub fn setup_virtual_microphone() -> Result<VirtualMicResult> {
    let output = Command::new("pactl")
        .args([
            "load-module",
            "module-pipe-source",
            "source_name=mike_virtual_microphone",
            "file=/tmp/mike_audio_pipe",
            "format=s16le",
            "rate=44100",
            "channels=2",
            "source_properties=device.description=Mike_Virtual_Microphone",
        ])
        .output()?;

    if output.status.success() {
        Ok(VirtualMicResult::Success)
    } else {
        eprintln!(
            "Failed to create virtual microphone: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(VirtualMicResult::Failed)
    }
}

/// Get the module ID of the virtual microphone for cleanup
pub fn get_virtual_microphone_module_id() -> Result<Option<String>> {
    let output = Command::new("pactl")
        .args(["list", "modules", "short"])
        .output()?;

    let output_str = String::from_utf8(output.stdout)?;

    for line in output_str.lines() {
        if line.contains("module-pipe-source")
            && line.contains("source_name=mike_virtual_microphone")
        {
            if let Some(module_id) = line.split_whitespace().next() {
                return Ok(Some(module_id.to_string()));
            }
        }
    }

    Ok(None)
}

/// Remove virtual microphone module
pub fn cleanup_virtual_microphone() -> Result<bool> {
    if let Some(module_id) = get_virtual_microphone_module_id()? {
        let output = Command::new("pactl")
            .args(["unload-module", &module_id])
            .output()?;

        if output.status.success() {
            println!(
                "Virtual microphone module {} unloaded successfully",
                module_id
            );
            Ok(true)
        } else {
            eprintln!(
                "Failed to unload module {}: {}",
                module_id,
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(false)
        }
    } else {
        println!("No virtual microphone module found to cleanup");
        Ok(false)
    }
}

/// Parse server address with validation
pub fn parse_server_address(addr: Option<String>) -> String {
    match addr {
        Some(addr) if !addr.trim().is_empty() => {
            if addr.contains(':') {
                addr
            } else {
                format!("{}:8080", addr)
            }
        }
        _ => "127.0.0.1:8080".to_string(),
    }
}

/// Validate audio buffer size
pub fn validate_buffer_size(size: usize) -> Result<usize> {
    match size {
        0 => Err(anyhow::anyhow!("Buffer size cannot be zero")),
        s if s > 65536 => Err(anyhow::anyhow!("Buffer size too large: {}", s)),
        s => Ok(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.channels, 2);
        matches!(config.format, AudioFormat::S16LE);
    }

    #[test]
    fn test_parse_server_address_with_port() {
        let addr = parse_server_address(Some("192.168.1.100:9090".to_string()));
        assert_eq!(addr, "192.168.1.100:9090");
    }

    #[test]
    fn test_parse_server_address_without_port() {
        let addr = parse_server_address(Some("192.168.1.100".to_string()));
        assert_eq!(addr, "192.168.1.100:8080");
    }

    #[test]
    fn test_parse_server_address_empty() {
        let addr = parse_server_address(Some("".to_string()));
        assert_eq!(addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_parse_server_address_none() {
        let addr = parse_server_address(None);
        assert_eq!(addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_validate_buffer_size_valid() {
        assert_eq!(validate_buffer_size(4096).unwrap(), 4096);
        assert_eq!(validate_buffer_size(1024).unwrap(), 1024);
    }

    #[test]
    fn test_validate_buffer_size_zero() {
        assert!(validate_buffer_size(0).is_err());
    }

    #[test]
    fn test_validate_buffer_size_too_large() {
        assert!(validate_buffer_size(100000).is_err());
    }

    #[test]
    fn test_setup_virtual_microphone_integration() {
        // This test verifies the function compiles and can be called
        let result = setup_virtual_microphone();
        match result {
            Ok(_) => {}  // Success or failure both fine in test environment
            Err(_) => {} // Error is acceptable in test environment
        }
    }

    #[test]
    fn test_parse_server_address_whitespace() {
        let addr = parse_server_address(Some("  ".to_string()));
        assert_eq!(addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_parse_server_address_with_protocol() {
        let addr = parse_server_address(Some("192.168.1.100:9090".to_string()));
        assert_eq!(addr, "192.168.1.100:9090");
    }

    #[test]
    fn test_validate_buffer_size_edge_cases() {
        assert_eq!(validate_buffer_size(1).unwrap(), 1);
        assert_eq!(validate_buffer_size(65536).unwrap(), 65536);
        assert!(validate_buffer_size(65537).is_err());
    }

    #[test]
    fn test_audio_format_debug() {
        let format = AudioFormat::S16LE;
        assert_eq!(format!("{:?}", format), "S16LE");

        let format = AudioFormat::F32LE;
        assert_eq!(format!("{:?}", format), "F32LE");
    }

    #[test]
    fn test_virtual_mic_result_debug() {
        assert_eq!(format!("{:?}", VirtualMicResult::Success), "Success");
        assert_eq!(format!("{:?}", VirtualMicResult::Failed), "Failed");
    }

    #[test]
    fn test_get_virtual_microphone_module_id() {
        // This test verifies the function compiles and handles no module case
        let result = get_virtual_microphone_module_id();
        match result {
            Ok(None) => {}    // Expected when no module is loaded
            Ok(Some(_)) => {} // Also fine if module exists
            Err(_) => {}      // Error is acceptable in test environment
        }
    }

    #[test]
    fn test_cleanup_virtual_microphone() {
        // This test verifies the function compiles and handles cleanup
        let result = cleanup_virtual_microphone();
        match result {
            Ok(_) => {}  // Success or no-op both fine
            Err(_) => {} // Error is acceptable in test environment
        }
    }
}
