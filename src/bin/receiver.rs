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

fn main() -> anyhow::Result<()> {
    println!("Virtual microphone server starting...");

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
            if Path::new("/tmp/mike_audio_pipe").exists() {
                if let Err(e) = std::fs::remove_file("/tmp/mike_audio_pipe") {
                    eprintln!("Error removing audio pipe: {}", e);
                }
            }

            r.store(false, Ordering::SeqCst);
            std::process::exit(0);
        }
    });

    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Server listening on port 8080...");
    println!("Virtual microphone 'mike_virtual_microphone' created");
    println!("Remote desktop software can now use this as a microphone input");
    println!("Press Ctrl+C to stop and cleanup");

    for stream in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let stream = stream?;

        thread::spawn(move || {
            if let Err(e) = handle_audio_stream(stream) {
                eprintln!("Error handling audio stream: {}", e);
            }
        });
    }

    Ok(())
}

fn handle_audio_stream(mut tcp_stream: TcpStream) -> anyhow::Result<()> {
    let fifo_path = "/tmp/mike_audio_pipe";

    if Path::new(fifo_path).exists() {
        std::fs::remove_file(fifo_path)?;
    }

    let status = Command::new("mkfifo").arg(fifo_path).status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create audio pipe"));
    }

    println!("Audio pipe created at {}", fifo_path);
    println!("Virtual microphone will read audio data from this pipe");

    let buffer_size = validate_buffer_size(4096)?;
    let mut buffer = vec![0u8; buffer_size];

    let pipe_writer = thread::spawn(move || -> anyhow::Result<()> {
        let mut fifo = OpenOptions::new().write(true).open(fifo_path)?;

        loop {
            match tcp_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("Client disconnected");
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
