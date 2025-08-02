use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let config = device.default_output_config().unwrap();

    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();

    let listener = TcpListener::bind("0.0.0.0:8080").expect("failed to bind server port");
    println!("Server listening for audio connections...");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let device = device.clone();
        let config = config.clone();

        thread::spawn(move || {
            let mut stream = stream;
            match sample_format {
                cpal::SampleFormat::F32 => play_audio::<f32>(&device, &config, &mut stream),
                cpal::SampleFormat::I16 => play_audio::<i16>(&device, &config, &mut stream),
                cpal::SampleFormat::U16 => play_audio::<u16>(&device, &config, &mut stream),
                _ => eprintln!("Unsupported sample format"),
            }
        });
    }
}

fn play_audio<T>(device: &cpal::Device, config: &cpal::StreamConfig, stream: &mut TcpStream)
where
    T: cpal::Sample + Default + cpal::SizedSample + Send + 'static,
{
    let buffer = Arc::new(Mutex::new(vec![T::default(); 1024]));
    let buffer_clone = buffer.clone();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let output_stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                if let Ok(buffer) = buffer_clone.lock() {
                    let len = data.len().min(buffer.len());
                    data[..len].copy_from_slice(&buffer[..len]);
                }
            },
            err_fn,
            None,
        )
        .unwrap();

    output_stream.play().unwrap();

    // Read audio data and update buffer
    let mut raw = vec![0u8; 1024 * std::mem::size_of::<T>()];
    loop {
        if let Ok(n) = stream.read(&mut raw) {
            if n == 0 {
                break;
            }
            if let Ok(mut buffer) = buffer.lock() {
                let expected_samples = n / std::mem::size_of::<T>();
                if n % std::mem::size_of::<T>() == 0
                    && expected_samples <= raw.len() / std::mem::size_of::<T>()
                {
                    let samples = unsafe {
                        std::slice::from_raw_parts(raw.as_ptr() as *const T, expected_samples)
                    };
                    let copy_len = samples.len().min(buffer.len());
                    buffer[..copy_len].copy_from_slice(&samples[..copy_len]);
                }
            }
        } else {
            break;
        }
    }
}
