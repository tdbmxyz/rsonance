use clap::{Parser, Subcommand};

/// Rsonance - Audio Transmission Tool
/// 
/// A Rust tool that captures microphone audio and transmits it to another device for playback
/// through a virtual audio input device. This enables remote desktop software to capture audio
/// from a remote microphone.
#[derive(Parser)]
#[command(name = "rsonance")]
#[command(about = "Audio transmission tool for remote microphone streaming")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a virtual microphone and receive audio streams
    Receiver {
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
        #[arg(short, long, default_value = "rsonance_virtual_microphone")]
        microphone_name: String,

        /// FIFO pipe path for audio data
        #[arg(short, long, default_value = "/tmp/rsonance_audio_pipe")]
        fifo_path: String,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Stream microphone audio to a remote virtual microphone
    Transmitter {
        /// Server address to connect to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Server port to connect to
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// Audio buffer size in bytes (affects latency)
        #[arg(short, long, default_value_t = 4096)]
        buffer_size: usize,

        /// Reconnection attempts on connection failure
        #[arg(short, long, default_value_t = 5)]
        reconnect_attempts: u32,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::init();
    
    let cli = Cli::parse();

    match cli.command {
        Commands::Receiver {
            host,
            port,
            buffer_size,
            microphone_name,
            fifo_path,
            verbose,
        } => {
            rsonance::receiver::run_receiver(
                host, port, buffer_size, microphone_name, fifo_path, verbose
            )
        }
        Commands::Transmitter {
            host,
            port,
            buffer_size,
            reconnect_attempts,
            verbose,
        } => {
            rsonance::transmitter::run_transmitter(
                host, port, buffer_size, reconnect_attempts, verbose
            ).await
        }
    }
}
