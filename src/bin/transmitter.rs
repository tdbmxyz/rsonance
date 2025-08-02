use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use mike::parse_server_address;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = parse_server_address(std::env::args().nth(1));

    println!("Connecting to server at {}...", server_addr);

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

    let config = device.default_input_config()?;
    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();

    println!(
        "Using audio format: {:?} at {} Hz with {} channels",
        sample_format, config.sample_rate.0, config.channels
    );

    let tcp_stream = TcpStream::connect(&server_addr).await?;
    println!("Connected to server successfully");

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let err_fn = |err| eprintln!("Audio stream error: {}", err);

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
    const MAX_RECONNECT_ATTEMPTS: u32 = 5;

    loop {
        tokio::select! {
            data = rx.recv() => {
                match data {
                    Some(audio_data) => {
                        if let Err(e) = tcp_stream.write_all(&audio_data).await {
                            eprintln!("Failed to send audio data: {}", e);

                            if reconnect_attempts < MAX_RECONNECT_ATTEMPTS {
                                println!("Attempting to reconnect... ({}/{})",
                                        reconnect_attempts + 1, MAX_RECONNECT_ATTEMPTS);

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
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            let bytes = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
            };

            if let Err(e) = tx.send(bytes.to_vec()) {
                eprintln!("Failed to send audio data to channel: {}", e);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
