use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use mike::validate_buffer_size;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Audio transmitter that captures microphone input and streams it to a remote receiver
#[derive(Parser)]
#[command(name = "mike-transmitter")]
#[command(about = "Stream microphone audio to a remote virtual microphone")]
#[command(version)]
struct Args {
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server_addr = format!("{}:{}", args.host, args.port);

    // Validate buffer size
    validate_buffer_size(args.buffer_size)?;

    println!("Connecting to server at {}...", server_addr);

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

    let config = device.default_input_config()?;
    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();

    if args.verbose {
        println!(
            "Using audio format: {:?} at {} Hz with {} channels",
            sample_format, config.sample_rate.0, config.channels
        );
        println!("Buffer size: {} bytes", args.buffer_size);
        println!("Max reconnection attempts: {}", args.reconnect_attempts);
    }

    let tcp_stream = TcpStream::connect(&server_addr).await?;
    println!("Connected to server successfully");

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let verbose_err = args.verbose;
    let err_fn = move |err| {
        if verbose_err {
            eprintln!("Audio stream error: {}", err);
        }
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => build_input_stream::<f32>(&device, &config, tx, err_fn)?,
        cpal::SampleFormat::I16 => build_input_stream::<i16>(&device, &config, tx, err_fn)?,
        cpal::SampleFormat::U16 => build_input_stream::<u16>(&device, &config, tx, err_fn)?,
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported sample format: {:?}",
                sample_format
            ));
        }
    };

    stream.play()?;
    println!("Started streaming microphone audio... Press Ctrl+C to stop.");

    let mut tcp_stream = tcp_stream;
    let mut reconnect_attempts = 0;
    let max_reconnect_attempts = args.reconnect_attempts;

    loop {
        tokio::select! {
            data = rx.recv() => {
                match data {
                    Some(audio_data) => {
                        if let Err(e) = tcp_stream.write_all(&audio_data).await {
                            eprintln!("Failed to send audio data: {}", e);

                            if reconnect_attempts < max_reconnect_attempts {
                                println!("Attempting to reconnect... ({}/{})",
                                        reconnect_attempts + 1, max_reconnect_attempts);

                                match TcpStream::connect(&server_addr).await {
                                    Ok(new_stream) => {
                                        tcp_stream = new_stream;
                                        reconnect_attempts = 0;
                                        println!("Reconnected successfully");
                                    }
                                    Err(e) => {
                                        eprintln!("Reconnection failed: {}", e);
                                        reconnect_attempts += 1;
                                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                    }
                                }
                            } else {
                                return Err(anyhow::anyhow!("Max reconnection attempts reached"));
                            }
                        } else {
                            reconnect_attempts = 0;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + Send + 'static,
{
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    
    let packet_count = Arc::new(AtomicUsize::new(0));
    let packet_count_clone = packet_count.clone();
    
    // Print debug info every 100 packets (about every 2 seconds at typical rates)
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let count = packet_count_clone.load(Ordering::Relaxed);
            println!("Audio packets captured: {} (in last 5 seconds)", count);
            packet_count_clone.store(0, Ordering::Relaxed);
        }
    });

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            // Convert audio data to S16LE format for PulseAudio compatibility
            let converted_data = convert_to_s16le(data);
            
            packet_count.fetch_add(1, Ordering::Relaxed);

            if let Err(e) = tx.send(converted_data) {
                eprintln!("Failed to send audio data to channel: {}", e);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

/// Convert audio samples to S16LE format for PulseAudio compatibility
fn convert_to_s16le<T>(data: &[T]) -> Vec<u8>
where
    T: cpal::Sample + cpal::SizedSample + 'static,
{
    let mut result = Vec::with_capacity(data.len() * 2); // S16 is 2 bytes per sample
    
    for &sample in data {
        // Convert to i16 based on sample type
        let i16_sample = if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>() {
            // F32 to I16
            let f32_val = unsafe { *(&sample as *const T as *const f32) };
            (f32_val.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<i16>() {
            // I16 to I16 (no conversion needed)
            unsafe { *(&sample as *const T as *const i16) }
        } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>() {
            // U16 to I16
            let u16_val = unsafe { *(&sample as *const T as *const u16) };
            (u16_val as i32 - 32768) as i16
        } else {
            0i16 // Fallback for unsupported types
        };
        
        // Write as little-endian bytes
        result.extend_from_slice(&i16_sample.to_le_bytes());
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_f32_to_s16le() {
        let f32_data: &[f32] = &[0.0, 0.5, -0.5, 1.0, -1.0];
        let result = convert_to_s16le(f32_data);
        
        // Each f32 sample becomes 2 bytes (i16)
        assert_eq!(result.len(), f32_data.len() * 2);
        
        // Check specific conversions
        let samples: Vec<i16> = result
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        assert_eq!(samples[0], 0);              // 0.0 -> 0
        assert_eq!(samples[1], 16383);          // 0.5 -> ~half of i16::MAX
        assert_eq!(samples[2], -16383);         // -0.5 -> ~half of i16::MIN
        assert_eq!(samples[3], 32767);          // 1.0 -> i16::MAX
        assert_eq!(samples[4], -32767);         // -1.0 -> close to i16::MIN
    }

    #[test]
    fn test_convert_i16_to_s16le() {
        let i16_data: &[i16] = &[0, 1000, -1000, i16::MAX, i16::MIN];
        let result = convert_to_s16le(i16_data);
        
        assert_eq!(result.len(), i16_data.len() * 2);
        
        // Should be unchanged (i16 to i16)
        let samples: Vec<i16> = result
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        assert_eq!(samples, i16_data);
    }

    #[test]
    fn test_convert_u16_to_s16le() {
        let u16_data: &[u16] = &[0, 32768, 65535];
        let result = convert_to_s16le(u16_data);
        
        assert_eq!(result.len(), u16_data.len() * 2);
        
        let samples: Vec<i16> = result
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // u16 0 -> i16 -32768 (0 - 32768)
        // u16 32768 -> i16 0 (32768 - 32768)  
        // u16 65535 -> i16 32767 (65535 - 32768)
        assert_eq!(samples[0], -32768);
        assert_eq!(samples[1], 0);
        assert_eq!(samples[2], 32767);
    }

    #[test]
    fn test_convert_empty_data() {
        let empty_f32: &[f32] = &[];
        let result = convert_to_s16le(empty_f32);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_convert_f32_clamping() {
        // Test values outside [-1.0, 1.0] range
        let f32_data: &[f32] = &[2.0, -2.0, f32::INFINITY, f32::NEG_INFINITY];
        let result = convert_to_s16le(f32_data);
        
        let samples: Vec<i16> = result
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // All should be clamped to i16::MAX or i16::MIN
        assert_eq!(samples[0], i16::MAX);   // 2.0 clamped to 1.0
        assert_eq!(samples[1], -32767);    // -2.0 clamped to -1.0
        assert_eq!(samples[2], i16::MAX);   // Infinity clamped to 1.0
        assert_eq!(samples[3], -32767);    // -Infinity clamped to -1.0
    }
}
