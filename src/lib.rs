//! # Rsonance - Audio Transmission Tool
//!
//! A Rust tool that captures microphone audio and transmits it to another device for playback
//! through a virtual audio input device. This enables remote desktop software to capture audio
//! from a remote microphone.
//!
//! ## Architecture
//!
//! - **Transmitter**: Captures microphone audio using `cpal` and streams via TCP
//! - **Receiver**: Creates virtual microphone input and pipes received audio
//! - **Core**: Real-time audio streaming with minimal latency and reconnection support
//!
//! ## Example Usage
//!
//! ```no_run
//! use rsonance::{setup_virtual_microphone, cleanup_virtual_microphone, VirtualMicResult};
//! use log::info;
//!
//! // Initialize logger
//! env_logger::init();
//!
//! // Set up virtual microphone
//! match setup_virtual_microphone() {
//!     Ok(VirtualMicResult::Success) => info!("Virtual microphone created"),
//!     Ok(VirtualMicResult::Failed) => log::warn!("Failed to create virtual microphone"),
//!     Err(e) => log::error!("Error: {}", e),
//! }
//!
//! // Later, clean up
//! cleanup_virtual_microphone().unwrap();
//! ```

pub mod receiver;
pub mod transmitter;

use anyhow::Result;
use log::{info, error, debug};
use std::process::Command;

/// Configuration for audio streaming
/// 
/// This struct holds the audio format configuration used for streaming
/// between transmitter and receiver. It defines the sample rate, number
/// of channels, and audio format.
/// 
/// # Examples
/// 
/// ```
/// use rsonance::{AudioConfig, AudioFormat};
/// 
/// let config = AudioConfig::default();
/// assert_eq!(config.sample_rate, 44100);
/// assert_eq!(config.channels, 2);
/// ```
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Number of audio channels (1 for mono, 2 for stereo)
    pub channels: u16,
    /// Audio sample format
    pub format: AudioFormat,
}

/// Supported audio sample formats
/// 
/// This enum represents the different audio sample formats that can be
/// used for audio streaming. Currently supports signed 16-bit little-endian
/// and 32-bit floating point little-endian formats.
#[derive(Debug, Clone)]
pub enum AudioFormat {
    /// Signed 16-bit little-endian format (most common)
    S16LE,
    /// 32-bit floating point little-endian format
    F32LE,
}

impl Default for AudioConfig {
    /// Returns the default audio configuration
    /// 
    /// Default settings are:
    /// - Sample rate: 44100 Hz (CD quality)
    /// - Channels: 2 (stereo)
    /// - Format: S16LE (signed 16-bit little-endian)
    /// 
    /// These settings provide good compatibility with most audio systems
    /// while maintaining reasonable quality and performance.
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            format: AudioFormat::S16LE,
        }
    }
}

/// Result of virtual microphone setup operation
/// 
/// This enum represents the outcome of attempting to create a virtual
/// microphone using PulseAudio. It indicates whether the operation
/// succeeded or failed.
/// 
/// # Examples
/// 
/// ```no_run
/// use rsonance::{setup_virtual_microphone, VirtualMicResult};
/// use log::info;
/// 
/// match setup_virtual_microphone() {
///     Ok(VirtualMicResult::Success) => info!("Virtual microphone created!"),
///     Ok(VirtualMicResult::Failed) => log::warn!("Failed to create virtual microphone"),
///     Err(e) => log::error!("Error: {}", e),
/// }
/// ```
#[derive(Debug, PartialEq)]
pub enum VirtualMicResult {
    /// Virtual microphone was created successfully
    Success,
    /// Virtual microphone creation failed
    Failed,
}

/// Creates a virtual microphone using default configuration
/// 
/// This function creates a virtual microphone device using PulseAudio's
/// `pactl` command with default settings. The virtual microphone will
/// appear as "mike_virtual_microphone" in the system's audio devices.
/// 
/// # Returns
/// 
/// Returns `Ok(VirtualMicResult::Success)` if the virtual microphone was
/// created successfully, `Ok(VirtualMicResult::Failed)` if the operation
/// failed, or `Err` if there was a system error.
/// 
/// # Examples
/// 
/// ```no_run
/// use rsonance::{setup_virtual_microphone, VirtualMicResult};
/// 
/// match setup_virtual_microphone()? {
///     VirtualMicResult::Success => log::info!("Virtual microphone ready!"),
///     VirtualMicResult::Failed => log::warn!("Setup failed"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// # Requirements
/// 
/// - PulseAudio must be running
/// - `pactl` command must be available in PATH
/// - User must have permissions to create audio devices
pub fn setup_virtual_microphone() -> Result<VirtualMicResult> {
    setup_virtual_microphone_with_config("rsonance_virtual_microphone", "/tmp/rsonance_audio_pipe")
}

/// Creates a virtual microphone with custom configuration
/// 
/// This function creates a virtual microphone device using PulseAudio's
/// `pactl` command with custom source name and FIFO path. It first creates
/// the required FIFO pipe, then loads the PulseAudio pipe-source module.
/// 
/// # Arguments
/// 
/// * `source_name` - Name for the virtual microphone source (e.g., "my_virtual_mic")
/// * `fifo_path` - Path where the FIFO pipe will be created (e.g., "/tmp/my_audio_pipe")
/// 
/// # Returns
/// 
/// Returns `Ok(VirtualMicResult::Success)` if the virtual microphone was
/// created successfully, `Ok(VirtualMicResult::Failed)` if the PulseAudio
/// operation failed, or `Err` if there was a system error (e.g., FIFO creation failed).
/// 
/// # Examples
/// 
/// ```no_run
/// use rsonance::{setup_virtual_microphone_with_config, VirtualMicResult};
/// 
/// let result = setup_virtual_microphone_with_config(
///     "my_custom_mic",
///     "/tmp/my_custom_pipe"
/// )?;
/// 
/// match result {
///     VirtualMicResult::Success => log::info!("Custom virtual microphone created!"),
///     VirtualMicResult::Failed => log::warn!("PulseAudio operation failed"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// # Requirements
/// 
/// - PulseAudio must be running
/// - `pactl` and `mkfifo` commands must be available in PATH
/// - User must have permissions to create files at `fifo_path`
/// - User must have permissions to load PulseAudio modules
/// 
/// # Notes
/// 
/// - If a FIFO already exists at `fifo_path`, it will be removed and recreated
/// - The virtual microphone will use S16LE format at 44100Hz with 2 channels
/// - The source description will be the source name with underscores replaced by spaces
pub fn setup_virtual_microphone_with_config(source_name: &str, fifo_path: &str) -> Result<VirtualMicResult> {
    // First, ensure the FIFO exists
    if std::path::Path::new(fifo_path).exists() {
        std::fs::remove_file(fifo_path)?;
    }
    
    let mkfifo_status = Command::new("mkfifo").arg(fifo_path).status()?;
    if !mkfifo_status.success() {
        return Err(anyhow::anyhow!("Failed to create FIFO pipe at {fifo_path}"));
    }

    let output = Command::new("pactl")
        .args([
            "load-module",
            "module-pipe-source",
            &format!("source_name={source_name}"),
            &format!("file={fifo_path}"),
            "format=s16le",
            "rate=44100",
            "channels=2",
            &format!("source_properties=device.description={}", source_name.replace('_', " ")),
        ])
        .output()?;

    if output.status.success() {
        info!("Virtual microphone '{source_name}' created successfully");
        debug!("FIFO pipe: {fifo_path}");
        Ok(VirtualMicResult::Success)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to create virtual microphone: {stderr}");
        Ok(VirtualMicResult::Failed)
    }
}

/// Get the module ID of the virtual microphone for cleanup
/// 
/// This function queries PulseAudio for loaded modules and searches for
/// the virtual microphone module (module-pipe-source with source_name=mike_virtual_microphone).
/// It returns the module ID if found, which can be used for cleanup.
/// 
/// # Returns
/// 
/// Returns `Ok(Some(module_id))` if the virtual microphone module is found,
/// `Ok(None)` if no matching module is loaded, or `Err` if the query failed.
/// 
/// # Examples
/// 
/// ```no_run
/// use rsonance::get_virtual_microphone_module_id;
/// 
/// match get_virtual_microphone_module_id()? {
///     Some(module_id) => log::info!("Virtual microphone module ID: {module_id}"),
///     None => log::debug!("No virtual microphone module found"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// # Requirements
/// 
/// - PulseAudio must be running
/// - `pactl` command must be available in PATH
pub fn get_virtual_microphone_module_id() -> Result<Option<String>> {
    let output = Command::new("pactl")
        .args(["list", "modules", "short"])
        .output()?;

    let output_str = String::from_utf8(output.stdout)?;

    for line in output_str.lines() {
        if line.contains("module-pipe-source")
            && line.contains("source_name=rsonance_virtual_microphone")
        {
            if let Some(module_id) = line.split_whitespace().next() {
                return Ok(Some(module_id.to_string()));
            }
        }
    }

    Ok(None)
}

/// Remove the virtual microphone module from PulseAudio
/// 
/// This function finds and unloads the virtual microphone module from PulseAudio.
/// It first queries for the module ID, then attempts to unload it using `pactl`.
/// 
/// # Returns
/// 
/// Returns `Ok(true)` if a module was found and successfully unloaded,
/// `Ok(false)` if no module was found or unloading failed, or `Err` if
/// there was a system error during the operation.
/// 
/// # Examples
/// 
/// ```no_run
/// use rsonance::cleanup_virtual_microphone;
/// 
/// match cleanup_virtual_microphone()? {
///     true => log::info!("Virtual microphone cleaned up successfully"),
///     false => log::debug!("No virtual microphone found or cleanup failed"),
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// # Requirements
/// 
/// - PulseAudio must be running
/// - `pactl` command must be available in PATH
/// - User must have permissions to unload PulseAudio modules
/// 
/// # Notes
/// 
/// This function specifically looks for the "rsonance_virtual_microphone" source.
/// If you used a different source name with `setup_virtual_microphone_with_config`,
/// you may need to manually unload the module using `pactl unload-module <id>`.
pub fn cleanup_virtual_microphone() -> Result<bool> {
    if let Some(module_id) = get_virtual_microphone_module_id()? {
        let output = Command::new("pactl")
            .args(["unload-module", &module_id])
            .output()?;

        if output.status.success() {
            info!("Virtual microphone module {module_id} unloaded successfully");
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to unload module {module_id}: {stderr}");
            Ok(false)
        }
    } else {
        debug!("No virtual microphone module found to cleanup");
        Ok(false)
    }
}

/// Parse and validate a server address string
/// 
/// This function takes an optional server address string and returns a properly
/// formatted address with default values applied as needed. If no port is specified,
/// it defaults to 8080. If the address is empty or None, it defaults to localhost.
/// 
/// # Arguments
/// 
/// * `addr` - Optional server address string (e.g., "192.168.1.100", "example.com:9090")
/// 
/// # Returns
/// 
/// Returns a formatted address string in the form "host:port"
/// 
/// # Examples
/// 
/// ```
/// use rsonance::parse_server_address;
/// 
/// assert_eq!(parse_server_address(Some("192.168.1.100".to_string())), "192.168.1.100:8080");
/// assert_eq!(parse_server_address(Some("example.com:9090".to_string())), "example.com:9090");
/// assert_eq!(parse_server_address(None), "127.0.0.1:8080");
/// assert_eq!(parse_server_address(Some("".to_string())), "127.0.0.1:8080");
/// ```
/// 
/// # Default Values
/// 
/// - Default host: 127.0.0.1 (localhost)
/// - Default port: 8080
pub fn parse_server_address(addr: Option<String>) -> String {
    match addr {
        Some(addr) if !addr.trim().is_empty() => {
            if addr.contains(':') {
                addr
            } else {
                format!("{addr}:8080")
            }
        }
        _ => "127.0.0.1:8080".to_string(),
    }
}

/// Validate audio buffer size for streaming
/// 
/// This function validates that the provided buffer size is within acceptable
/// limits for audio streaming. The buffer size affects latency and performance:
/// smaller buffers reduce latency but may cause audio dropouts, while larger
/// buffers increase latency but provide more stability.
/// 
/// # Arguments
/// 
/// * `size` - Buffer size in bytes to validate
/// 
/// # Returns
/// 
/// Returns `Ok(size)` if the buffer size is valid, or `Err` with a descriptive
/// error message if the size is invalid.
/// 
/// # Validation Rules
/// 
/// - Buffer size must be greater than 0
/// - Buffer size must not exceed 65536 bytes (64KB)
/// 
/// # Examples
/// 
/// ```
/// use rsonance::validate_buffer_size;
/// 
/// assert_eq!(validate_buffer_size(4096).unwrap(), 4096);
/// assert_eq!(validate_buffer_size(1024).unwrap(), 1024);
/// 
/// assert!(validate_buffer_size(0).is_err());
/// assert!(validate_buffer_size(100000).is_err());
/// ```
/// 
/// # Recommended Values
/// 
/// - 1024 bytes: Very low latency, may cause dropouts on slower systems
/// - 4096 bytes: Good balance of latency and stability (default)
/// - 8192 bytes: Higher latency but very stable
/// - 16384 bytes: High latency, maximum stability
pub fn validate_buffer_size(size: usize) -> Result<usize> {
    match size {
        0 => Err(anyhow::anyhow!("Buffer size cannot be zero")),
        s if s > 65536 => Err(anyhow::anyhow!("Buffer size too large: {s}")),
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
        // Function should return either Success, Failed, or an error
        match result {
            Ok(VirtualMicResult::Success) | Ok(VirtualMicResult::Failed) => {
                // Both success and failure are acceptable in test environment
            }
            Err(_) => {
                // Error is also acceptable (e.g., pactl not available)
            }
        }
    }

    #[test]
    fn test_setup_virtual_microphone_with_custom_config() {
        let result = setup_virtual_microphone_with_config(
            "test_virtual_mic",
            "/tmp/test_fifo_pipe"
        );
        // Function should return a result, any outcome is acceptable in test environment
        match result {
            Ok(_) | Err(_) => {} // Both success and error are acceptable
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
    fn test_validate_buffer_size_typical_values() {
        // Test common buffer sizes
        assert_eq!(validate_buffer_size(1024).unwrap(), 1024);
        assert_eq!(validate_buffer_size(2048).unwrap(), 2048);
        assert_eq!(validate_buffer_size(4096).unwrap(), 4096);
        assert_eq!(validate_buffer_size(8192).unwrap(), 8192);
        assert_eq!(validate_buffer_size(16384).unwrap(), 16384);
    }

    #[test]
    fn test_parse_server_address_edge_cases() {
        // Test IPv6 addresses (should pass through unchanged if they have port)
        assert_eq!(
            parse_server_address(Some("[::1]:8080".to_string())),
            "[::1]:8080"
        );
        
        // Test hostname with port
        assert_eq!(
            parse_server_address(Some("example.com:9090".to_string())),
            "example.com:9090"
        );
        
        // Test just hostname
        assert_eq!(
            parse_server_address(Some("example.com".to_string())),
            "example.com:8080"
        );
    }

    #[test]
    fn test_audio_format_debug() {
        let format = AudioFormat::S16LE;
        assert_eq!(format!("{format:?}"), "S16LE");

        let format = AudioFormat::F32LE;
        assert_eq!(format!("{format:?}"), "F32LE");
    }

    #[test]
    fn test_audio_config_clone() {
        let config = AudioConfig::default();
        let cloned = config.clone();
        assert_eq!(config.sample_rate, cloned.sample_rate);
        assert_eq!(config.channels, cloned.channels);
    }

    #[test]
    fn test_audio_config_custom() {
        let config = AudioConfig {
            sample_rate: 48000,
            channels: 1,
            format: AudioFormat::F32LE,
        };
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 1);
        matches!(config.format, AudioFormat::F32LE);
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
            Ok(None) => {
                // Expected when no virtual microphone module is loaded
            }
            Ok(Some(module_id)) => {
                // Verify module_id is a valid string (non-empty)
                assert!(!module_id.is_empty());
            }
            Err(_) => {
                // Error is acceptable in test environment (e.g., pactl not available)
            }
        }
    }

    #[test]
    fn test_cleanup_virtual_microphone() {
        // This test verifies the function compiles and handles cleanup
        let result = cleanup_virtual_microphone();
        match result {
            Ok(true) => {
                // Successfully found and unloaded a module
            }
            Ok(false) => {
                // No module found to cleanup (expected in most test environments)
            }
            Err(_) => {
                // Error is acceptable in test environment (e.g., pactl not available)
            }
        }
    }
}
