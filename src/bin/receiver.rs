use clap::Parser;
use mike::{
    VirtualMicResult, cleanup_virtual_microphone, setup_virtual_microphone, validate_buffer_size,
};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
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
    let result = setup_virtual_microphone()?;
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
    if Path::new(&fifo_path).exists() {
        std::fs::remove_file(&fifo_path)?;
    }

    let status = Command::new("mkfifo").arg(&fifo_path).status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create audio pipe"));
    }

    if verbose {
        println!("Audio pipe created at {}", fifo_path);
        println!("Virtual microphone will read audio data from this pipe");
        println!("Using buffer size: {} bytes", buffer_size);
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
