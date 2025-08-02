use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device available");
    let config = device.default_input_config().unwrap();

    let stream = Arc::new(Mutex::new(None));
    let server_addr = "SERVER_IP:PORT"; // <-- change this

    let stream_clone = stream.clone();
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let sample_format = config.sample_format();
    let config = config.into();

    let mut tcp_stream = TcpStream::connect(server_addr).expect("failed to connect to server");

    let new_stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        data.as_ptr() as *const u8,
                        data.len() * std::mem::size_of::<f32>(),
                    )
                };
                let _ = tcp_stream.write_all(bytes);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        data.as_ptr() as *const u8,
                        data.len() * std::mem::size_of::<i16>(),
                    )
                };
                let _ = tcp_stream.write_all(bytes);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        data.as_ptr() as *const u8,
                        data.len() * std::mem::size_of::<u16>(),
                    )
                };
                let _ = tcp_stream.write_all(bytes);
            },
            err_fn,
            None,
        ),
    }.unwrap();

    *stream_clone.lock().unwrap() = Some(new_stream);

    stream_clone.lock().unwrap().as_ref().unwrap().play().unwrap();

    println!("Streaming audio to server at {server_addr}... Press Ctrl+C to stop.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}