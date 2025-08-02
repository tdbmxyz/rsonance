use pipewire as pw;
use std::io::Read;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u32 = 2;
const FRAME_SIZE: usize = (std::mem::size_of::<i16>() * CHANNELS as usize);

fn main() {
    // Init PipeWire
    pw::init();
    let mainloop = pw::MainLoop::new().expect("Failed to create mainloop");
    let context = pw::Context::new(&mainloop).expect("Failed to create context");
    let core = context.connect(None).expect("Failed to connect to PipeWire");

    // Create a source node (virtual mic)
    let stream = pw::stream::Stream::new(
        &core,
        "Network Audio Source",
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::DRIVER,
    )
    .expect("Failed to create stream");

    // Setup format: S16LE, 48khz, Stereo
    let format = pw::spa::format::AudioFormat::S16LE;
    let params = pw::params::audio::AudioInfoBuilder::default()
        .format(format)
        .rate(SAMPLE_RATE)
        .channels(CHANNELS)
        .build()
        .as_params();

    // Start stream
    stream
        .connect(
            pw::Direction::Output,
            None,
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::DRIVER,
            &params,
        )
        .expect("Failed to connect stream");

    println!("PipeWire virtual source node started.");

    // Networking: spawn TCP server
    let listener = TcpListener::bind("0.0.0.0:PORT").expect("Failed to bind TCP port"); // Change PORT
    println!("Server listening on 0.0.0.0:PORT");

    // Shared audio buffer for network -> PipeWire thread communication
    let audio_data = Arc::new(Mutex::new(Vec::<i16>::new()));

    // Handle incoming TCP in a thread
    let audio_data_cloned = Arc::clone(&audio_data);
    thread::spawn(move || {
        for conn in listener.incoming() {
            let mut stream = conn.expect("Failed to accept connection");
            println!("Client connected.");
            let mut buf = [0u8; 2048];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let samples: &[i16] = unsafe {
                            std::slice::from_raw_parts(buf.as_ptr() as *const i16, n / 2)
                        };
                        let mut audio = audio_data_cloned.lock().unwrap();
                        audio.extend_from_slice(samples);
                    }
                    Err(_) => break,
                }
            }
            println!("Client disconnected.");
        }
    });

    // Main PipeWire loop: feed data from audio buffer into PipeWire
    stream.set_process(move |stream| {
        let mut audio = audio_data.lock().unwrap();
        let mut buffer = stream.dequeue_buffer().expect("No buffer available");
        let mut data = buffer.data_mut();
        let frames = data.len() / FRAME_SIZE;
        let needed_samples = frames * CHANNELS as usize;
        let available = audio.len().min(needed_samples);

        // Fill buffer
        let sample_bytes = unsafe {
            std::slice::from_raw_parts(
                audio.as_ptr() as *const u8,
                available * std::mem::size_of::<i16>(),
            )
        };
        data[..sample_bytes.len()].copy_from_slice(sample_bytes);

        // Remove used samples
        audio.drain(..available);

        // Set buffer metadata
        buffer.set_length(available / CHANNELS as usize);

        Ok(())
    });

    // Run event loop
    mainloop.run();
}