use std::sync::{Arc, Mutex};

use cpal::traits::*;

pub fn start_audio() -> Result<AudioResult, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No output device available".to_string())?;
    let config = device.default_output_config().unwrap();
    let sample_format = config.sample_format();
    let mut config = config.config();

    // max channels: 2
    config.channels = config.channels.min(2);

    let proc = Arc::new(Mutex::new(Box::new(move |len| vec![0.0f32; len])
        as Box<dyn FnMut(usize) -> Vec<f32> + Send + Sync + 'static>));

    let stream = match sample_format {
        cpal::SampleFormat::F32 => run::<f32>(device, config.clone(), proc.clone()),
        cpal::SampleFormat::I16 => run::<i16>(device, config.clone(), proc.clone()),
        cpal::SampleFormat::U16 => run::<u16>(device, config.clone(), proc.clone()),
        _ => todo!(),
    };

    Ok(AudioResult {
        sample_rate: config.sample_rate.0 as usize,
        channels: config.channels as usize,
        callback: proc,
        stream,
    })
}

type ProcFn = Arc<Mutex<Box<dyn FnMut(usize) -> Vec<f32> + Send + Sync + 'static>>>;

pub struct AudioResult {
    pub sample_rate: usize,
    pub channels: usize,
    pub callback: ProcFn,
    stream: cpal::Stream,
}

fn run<T>(device: cpal::Device, config: cpal::StreamConfig, proc: ProcFn) -> cpal::Stream
where
    T: cpal::SizedSample + cpal::FromSample<f32> + 'static,
{
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let buf = proc.lock().unwrap()(data.len());
                for i in 0..data.len() {
                    data[i] = T::from_sample(buf[i]);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| e.to_string())
        .unwrap();
    stream.play().map_err(|e| e.to_string()).unwrap();
    stream
}

impl Drop for AudioResult {
    fn drop(&mut self) {
        self.stream.pause().unwrap();
    }
}
