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
    PipeWireSuccess,
    PulseAudioFallback,
    Failed,
}

/// Creates virtual microphone using PipeWire or PulseAudio fallback
pub fn setup_virtual_microphone_with_commands(
    pw_command: impl Fn(&str) -> Result<bool>,
    pa_command: impl Fn(&str) -> Result<bool>,
) -> Result<VirtualMicResult> {
    let pw_cmd = r#"
pw-cli create-node adapter \
    '{ factory.name=support.null-audio-sink \
       node.name=mike-virtual-source \
       node.description="Mike Virtual Microphone" \
       media.class=Audio/Source \
       audio.rate=44100 \
       audio.channels=2 \
       audio.format=S16LE }' 2>/dev/null || true
"#;

    if pw_command(pw_cmd.trim())? {
        return Ok(VirtualMicResult::PipeWireSuccess);
    }

    let pa_cmd = r#"
pactl load-module module-pipe-source \
    source_name=mike_virtual_microphone \
    file=/tmp/mike_audio_pipe \
    format=s16le \
    rate=44100 \
    channels=2 \
    source_properties=device.description="Mike_Virtual_Microphone" || true
"#;

    if pa_command(pa_cmd.trim())? {
        Ok(VirtualMicResult::PulseAudioFallback)
    } else {
        Ok(VirtualMicResult::Failed)
    }
}

/// Execute shell command and return success status
pub fn execute_shell_command(cmd: &str) -> Result<bool> {
    let status = Command::new("sh").arg("-c").arg(cmd).status()?;
    Ok(status.success())
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
    fn test_setup_virtual_microphone_pipewire_success() {
        let pw_command = |_: &str| Ok(true);
        let pa_command = |_: &str| Ok(false);

        let result = setup_virtual_microphone_with_commands(pw_command, pa_command).unwrap();
        assert_eq!(result, VirtualMicResult::PipeWireSuccess);
    }

    #[test]
    fn test_setup_virtual_microphone_pulseaudio_fallback() {
        let pw_command = |_: &str| Ok(false);
        let pa_command = |_: &str| Ok(true);

        let result = setup_virtual_microphone_with_commands(pw_command, pa_command).unwrap();
        assert_eq!(result, VirtualMicResult::PulseAudioFallback);
    }

    #[test]
    fn test_setup_virtual_microphone_failed() {
        let pw_command = |_: &str| Ok(false);
        let pa_command = |_: &str| Ok(false);

        let result = setup_virtual_microphone_with_commands(pw_command, pa_command).unwrap();
        assert_eq!(result, VirtualMicResult::Failed);
    }

    #[test]
    fn test_setup_virtual_microphone_error_handling() {
        let pw_command = |_: &str| Err(anyhow::anyhow!("Command failed"));
        let pa_command = |_: &str| Ok(true);

        let result = setup_virtual_microphone_with_commands(pw_command, pa_command);
        assert!(result.is_err());
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
        assert_eq!(
            format!("{:?}", VirtualMicResult::PipeWireSuccess),
            "PipeWireSuccess"
        );
        assert_eq!(
            format!("{:?}", VirtualMicResult::PulseAudioFallback),
            "PulseAudioFallback"
        );
        assert_eq!(format!("{:?}", VirtualMicResult::Failed), "Failed");
    }
}
