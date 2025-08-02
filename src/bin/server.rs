use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device available");
    let config = device.default_output_config().unwrap();

    let sample_format = config.sample_format();
    let config = config.into();

    let listener = TcpListener::bind("0.0.0.0:PORT").expect("failed to bind server port"); // <-- change this
    println!("Server listening for audio connections...");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let device = device.clone();
        let config = config.clone();

        thread::spawn(move || {
            match sample_format {
                cpal::SampleFormat::F32 => play_audio::<f32>(&device, &config, &mut stream),
                cpal::SampleFormat::I16 => play_audio::<i16>(&device, &config, &mut stream),
                cpal::SampleFormat::U16 => play_audio::<u16>(&device, &config, &mut stream),
            }
        });
    }
}

fn play_audio<T>(device: &cpal::Device, config: &cpal::StreamConfig, stream: &mut TcpStream)
where
    T: cpal::Sample + Default,
{
    let buffer = Arc::new(Mutex::new(vec![T::default(); 1024]));
    let buffer_clone = buffer.clone();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let output_stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let mut buffer = buffer_clone.lock().unwrap();
            data.copy_from_slice(&buffer[..data.len()]);
        },
        err_fn,
        None,
    ).unwrap();

    output_stream.play().unwrap();

    // Read audio data and update buffer
    let mut raw = vec![0u8; 1024 * std::mem::size_of::<T>()];
    loop {
        if let Ok(n) = stream.read(&mut raw) {
            if n == 0 { break; }
            let mut buffer = buffer.lock().unwrap();
            let samples = unsafe {
                std::slice::from_raw_parts(raw.as_ptr() as *const T, n / std::mem::size_of::<T>())
            };
            buffer[..samples.len()].copy_from_slice(samples);
        } else {
            break;
        }
    }
}