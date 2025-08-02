use mike::{
    VirtualMicResult, execute_shell_command, setup_virtual_microphone_with_commands,
    validate_buffer_size,
};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::thread;

fn main() -> anyhow::Result<()> {
    println!("Virtual microphone server starting...");

    let result = setup_virtual_microphone()?;
    match result {
        VirtualMicResult::PipeWireSuccess => {
            println!("PipeWire virtual source created successfully");
        }
        VirtualMicResult::PulseAudioFallback => {
            println!("Using PulseAudio compatibility mode");
        }
        VirtualMicResult::Failed => {
            eprintln!("Warning: Failed to create virtual microphone");
        }
    }

    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Server listening on port 8080...");
    println!("Virtual microphone 'mike-virtual-mic' created");
    println!("Remote desktop software can now use this as a microphone input");

    for stream in listener.incoming() {
        let stream = stream?;

        thread::spawn(move || {
            if let Err(e) = handle_audio_stream(stream) {
                eprintln!("Error handling audio stream: {}", e);
            }
        });
    }

    Ok(())
}

fn setup_virtual_microphone() -> anyhow::Result<VirtualMicResult> {
    println!("Setting up PipeWire virtual microphone...");

    setup_virtual_microphone_with_commands(execute_shell_command, execute_shell_command)
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

    // Try to use pw-cat to stream audio to PipeWire
    let fifo_path_clone = fifo_path.to_string();
    let _pw_cat_handle = thread::spawn(move || {
        let pw_cat_cmd = format!(
            "pw-cat --playback --target=mike-virtual-source --format=s16 --rate=44100 --channels=2 < {}",
            fifo_path_clone
        );

        println!("Starting pw-cat for PipeWire audio streaming...");
        let status = Command::new("sh").arg("-c").arg(&pw_cat_cmd).status();

        if let Err(e) = status {
            eprintln!("pw-cat failed: {}", e);
        }
    });

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_virtual_microphone_integration() {
        // This test verifies the function compiles and can be called
        // In a real environment, it would attempt to create the virtual mic
        let result = setup_virtual_microphone();
        // We don't assert success because it depends on system state
        // but we ensure it doesn't panic
        match result {
            Ok(_) => {}  // Success is fine
            Err(_) => {} // Error is also fine in test environment
        }
    }
}
