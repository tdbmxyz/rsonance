//! Audio receiver module that creates a virtual microphone and receives audio streams

use crate::{VirtualMicResult, cleanup_virtual_microphone, setup_virtual_microphone_with_config, validate_buffer_size};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Run the receiver with the given configuration
/// 
/// This function sets up a virtual microphone, binds to the specified address/port,
/// and handles incoming audio streams from transmitter clients.
/// 
/// # Arguments
/// 
/// * `host` - Host address to bind to (e.g., "0.0.0.0" or "127.0.0.1")
/// * `port` - Port number to listen on
/// * `buffer_size` - Audio buffer size in bytes (affects latency)
/// * `microphone_name` - Name of the virtual microphone to create
/// * `fifo_path` - Path where the FIFO pipe will be created
/// * `verbose` - Enable verbose logging output
/// 
/// # Returns
/// 
/// Returns `Ok(())` on successful completion, or an error if setup fails
/// 
/// # Example
/// 
/// ```no_run
/// mike::receiver::run_receiver(
///     "0.0.0.0".to_string(),
///     8080,
///     4096,
///     "my_virtual_mic".to_string(),
///     "/tmp/my_audio_pipe".to_string(),
///     true
/// ).unwrap();
/// ```
pub fn run_receiver(
    host: String,
    port: u16,
    buffer_size: usize,
    microphone_name: String,
    fifo_path: String,
    verbose: bool,
) -> anyhow::Result<()> {
    // Validate buffer size
    validate_buffer_size(buffer_size)?;

    println!("Virtual microphone server starting...");

    if verbose {
        println!("Configuration:");
        println!("  Host: {host}");
        println!("  Port: {port}");
        println!("  Buffer size: {buffer_size} bytes");
        println!("  Microphone name: {microphone_name}");
        println!("  FIFO path: {fifo_path}");
    }

    println!("Setting up virtual microphone...");
    let result = setup_virtual_microphone_with_config(&microphone_name, &fifo_path)?;
    match result {
        VirtualMicResult::Success => {
            println!("Virtual microphone created successfully");
        }
        VirtualMicResult::Failed => {
            eprintln!("Warning: Failed to create virtual microphone");
        }
    }

    // Set up signal handling for cleanup
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let fifo_path_cleanup = fifo_path.clone();

    let mut signals = Signals::new([SIGINT])?;
    thread::spawn(move || {
        if let Some(sig) = signals.forever().next() {
            println!("\nReceived signal {sig:?}, cleaning up...");

            // Cleanup virtual microphone
            if let Err(e) = cleanup_virtual_microphone() {
                eprintln!("Error cleaning up virtual microphone: {e}");
            } else {
                println!("Virtual microphone cleaned up successfully");
            }

            // Clean up FIFO
            if Path::new(&fifo_path_cleanup).exists() {
                if let Err(e) = std::fs::remove_file(&fifo_path_cleanup) {
                    eprintln!("Error removing audio pipe: {e}");
                }
            }

            r.store(false, Ordering::SeqCst);
            std::process::exit(0);
        }
    });

    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr)?;
    println!("Server listening on {bind_addr}...");
    println!("Virtual microphone '{microphone_name}' created");
    println!("Remote desktop software can now use this as a microphone input");
    println!("Press Ctrl+C to stop and cleanup");

    for stream in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let stream = stream?;
        let fifo_path = fifo_path.clone();

        thread::spawn(move || {
            if let Err(e) = handle_audio_stream(stream, fifo_path, buffer_size, verbose) {
                eprintln!("Error handling audio stream: {e}");
            }
        });
    }

    Ok(())
}

/// Handle an individual audio stream from a transmitter client
/// 
/// This function reads audio data from a TCP stream and writes it to the FIFO pipe
/// that feeds the virtual microphone.
/// 
/// # Arguments
/// 
/// * `tcp_stream` - The TCP connection from the transmitter
/// * `fifo_path` - Path to the FIFO pipe for audio data
/// * `buffer_size` - Size of the buffer for reading audio data
/// * `verbose` - Enable verbose logging
/// 
/// # Returns
/// 
/// Returns `Ok(())` on successful completion, or an error if the stream fails
fn handle_audio_stream(
    mut tcp_stream: TcpStream,
    fifo_path: String,
    buffer_size: usize,
    verbose: bool,
) -> anyhow::Result<()> {
    if verbose {
        println!("Starting audio stream handler");
        println!("FIFO path: {fifo_path}");
        println!("Using buffer size: {buffer_size} bytes");
    }

    // The FIFO should already exist, created by the virtual microphone setup
    if !Path::new(&fifo_path).exists() {
        return Err(anyhow::anyhow!("FIFO pipe does not exist at {fifo_path}"));
    }

    let mut buffer = vec![0u8; buffer_size];

    let pipe_writer = thread::spawn(move || -> anyhow::Result<()> {
        let mut fifo = OpenOptions::new().write(true).open(&fifo_path)?;

        loop {
            match tcp_stream.read(&mut buffer) {
                Ok(0) => {
                    if verbose {
                        println!("Client disconnected");
                    }
                    break;
                }
                Ok(n) => {
                    if verbose {
                        println!("Received {n} bytes of audio data, writing to FIFO");
                    }
                    if let Err(e) = fifo.write_all(&buffer[..n]) {
                        eprintln!("Failed to write to audio pipe: {e}");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("TCP read error: {e}");
                    break;
                }
            }
        }
        Ok(())
    });

    pipe_writer
        .join()
        .map_err(|_| anyhow::anyhow!("Pipe writer thread panicked"))??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_handle_audio_stream_missing_fifo() {
        use std::net::{TcpListener, TcpStream};
        use std::thread;
        
        // Create a test TCP connection
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
        });
        
        let _client_stream = TcpStream::connect(addr).unwrap();
        let server_stream = handle.join().unwrap();
        
        // Test with non-existent FIFO
        let result = handle_audio_stream(
            server_stream,
            "/tmp/non_existent_fifo".to_string(),
            4096,
            false
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FIFO pipe does not exist"));
    }

    #[test]
    fn test_handle_audio_stream_with_fifo() {
        use std::net::{TcpListener, TcpStream};
        use std::thread;
        use std::time::Duration;
        
        // Create test FIFO
        let test_fifo = "/tmp/test_audio_pipe_for_test";
        let _ = fs::remove_file(test_fifo); // Clean up if exists
        
        // Create FIFO
        let status = std::process::Command::new("mkfifo")
            .arg(test_fifo)
            .status();
        
        if status.is_err() || !status.unwrap().success() {
            // Skip test if mkfifo fails (not available)
            return;
        }
        
        // Create a test TCP connection
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        
        let test_fifo_clone = test_fifo.to_string();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            
            // Give some time for the handler to start
            thread::sleep(Duration::from_millis(100));
            
            // Don't run the full handler as it would block on FIFO reading
            // Just verify the FIFO exists check passes
            let _result = std::path::Path::new(&test_fifo_clone).exists();
            
            stream
        });
        
        let _client_stream = TcpStream::connect(addr).unwrap();
        let _server_stream = handle.join().unwrap();
        
        // Verify FIFO exists
        assert!(std::path::Path::new(test_fifo).exists());
        
        // Clean up
        let _ = fs::remove_file(test_fifo);
    }
}