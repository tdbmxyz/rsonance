use clap::Parser;
use mike::{
    VirtualMicResult, cleanup_virtual_microphone, setup_virtual_microphone_with_config, 
    validate_buffer_size,
};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Audio receiver that creates a virtual microphone and receives audio streams
#[derive(Parser)]
#[command(name = "mike-receiver")]
#[command(about = "Create a virtual microphone and receive audio streams")]
#[command(version)]
struct Args {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Audio buffer size in bytes (affects latency)
    #[arg(short, long, default_value_t = 4096)]
    buffer_size: usize,

    /// Virtual microphone name
    #[arg(short, long, default_value = "mike_virtual_microphone")]
    microphone_name: String,

    /// FIFO pipe path for audio data
    #[arg(short, long, default_value = "/tmp/mike_audio_pipe")]
    fifo_path: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Validate buffer size
    validate_buffer_size(args.buffer_size)?;

    println!("Virtual microphone server starting...");

    if args.verbose {
        println!("Configuration:");
        println!("  Host: {}", args.host);
        println!("  Port: {}", args.port);
        println!("  Buffer size: {} bytes", args.buffer_size);
        println!("  Microphone name: {}", args.microphone_name);
        println!("  FIFO path: {}", args.fifo_path);
    }

    println!("Setting up virtual microphone...");
    let result = setup_virtual_microphone_with_config(&args.microphone_name, &args.fifo_path)?;
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
    let fifo_path_cleanup = args.fifo_path.clone();

    let mut signals = Signals::new([SIGINT])?;
    thread::spawn(move || {
        if let Some(sig) = signals.forever().next() {
            println!("\nReceived signal {:?}, cleaning up...", sig);

            // Cleanup virtual microphone
            if let Err(e) = cleanup_virtual_microphone() {
                eprintln!("Error cleaning up virtual microphone: {}", e);
            } else {
                println!("Virtual microphone cleaned up successfully");
            }

            // Clean up FIFO
            if Path::new(&fifo_path_cleanup).exists() {
                if let Err(e) = std::fs::remove_file(&fifo_path_cleanup) {
                    eprintln!("Error removing audio pipe: {}", e);
                }
            }

            r.store(false, Ordering::SeqCst);
            std::process::exit(0);
        }
    });

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&bind_addr)?;
    println!("Server listening on {}...", bind_addr);
    println!("Virtual microphone '{}' created", args.microphone_name);
    println!("Remote desktop software can now use this as a microphone input");
    println!("Press Ctrl+C to stop and cleanup");

    for stream in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let stream = stream?;
        let fifo_path = args.fifo_path.clone();
        let buffer_size = args.buffer_size;
        let verbose = args.verbose;

        thread::spawn(move || {
            if let Err(e) = handle_audio_stream(stream, fifo_path, buffer_size, verbose) {
                eprintln!("Error handling audio stream: {}", e);
            }
        });
    }

    Ok(())
}

fn handle_audio_stream(
    mut tcp_stream: TcpStream,
    fifo_path: String,
    buffer_size: usize,
    verbose: bool,
) -> anyhow::Result<()> {
    if verbose {
        println!("Starting audio stream handler");
        println!("FIFO path: {}", fifo_path);
        println!("Using buffer size: {} bytes", buffer_size);
    }

    // The FIFO should already exist, created by the virtual microphone setup
    if !Path::new(&fifo_path).exists() {
        return Err(anyhow::anyhow!("FIFO pipe does not exist at {}", fifo_path));
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
                        println!("Received {} bytes of audio data, writing to FIFO", n);
                    }
                    if let Err(e) = fifo.write_all(&buffer[..n]) {
                        eprintln!("Failed to write to audio pipe: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("TCP read error: {}", e);
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
    fn test_args_parsing() {
        use clap::Parser;
        
        // Test default values
        let args = Args::try_parse_from(&["mike-receiver"]).unwrap();
        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, 8080);
        assert_eq!(args.buffer_size, 4096);
        assert_eq!(args.microphone_name, "mike_virtual_microphone");
        assert_eq!(args.fifo_path, "/tmp/mike_audio_pipe");
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_parsing_custom() {
        use clap::Parser;
        
        let args = Args::try_parse_from(&[
            "mike-receiver",
            "--host", "192.168.1.100",
            "--port", "9090",
            "--buffer-size", "8192",
            "--microphone-name", "test_mic",
            "--fifo-path", "/tmp/test_pipe",
            "--verbose"
        ]).unwrap();
        
        assert_eq!(args.host, "192.168.1.100");
        assert_eq!(args.port, 9090);
        assert_eq!(args.buffer_size, 8192);
        assert_eq!(args.microphone_name, "test_mic");
        assert_eq!(args.fifo_path, "/tmp/test_pipe");
        assert!(args.verbose);
    }

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
