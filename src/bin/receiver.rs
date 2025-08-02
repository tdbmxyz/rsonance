use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::thread;

fn main() -> anyhow::Result<()> {
    println!("Virtual microphone server starting...");

    setup_virtual_microphone()?;

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

fn setup_virtual_microphone() -> anyhow::Result<()> {
    let module_cmd = r#"
pactl load-module module-pipe-source \
    source_name=mike_virtual_microphone \
    file=/tmp/mike_audio_pipe \
    format=s16le \
    rate=44100 \
    channels=2 \
    source_properties=device.description="Mike_Virtual_Microphone" || true
"#;

    let status = Command::new("sh")
        .arg("-c")
        .arg(module_cmd.trim())
        .status()?;

    if !status.success() {
        println!("Warning: Could not load PulseAudio pipe source module");
        println!("Trying alternative approach...");

        let alt_cmd = r#"
pactl load-module module-null-sink \
    sink_name=mike_null_sink \
    sink_properties=device.description="Mike_Virtual_Sink"

pactl load-module module-remap-source \
    master=mike_null_sink.monitor \
    source_name=mike_virtual_microphone \
    source_properties=device.description="Mike_Virtual_Microphone"
"#;

        let _ = Command::new("sh").arg("-c").arg(alt_cmd.trim()).status();
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

    let mut buffer = vec![0u8; 4096];

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
