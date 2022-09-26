use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use std::sync::Arc;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;
use cpal::Stream;

fn run<'a, T>(
    device: &'a mut cpal::Device,
    config: &'a cpal::StreamConfig,
) -> (Arc<Stream>, f64, Receiver<bool>, Sender<(f64, f64)>)
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let (tx_req_sample, rx_req_sample): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    let (tx_sample, rx_sample): (Sender<(f64, f64)>, Receiver<(f64, f64)>) = mpsc::channel();

    let mut next_value = move || {
        tx_req_sample.send(true).unwrap_or_default();
        let res = rx_sample.recv().unwrap_or_default();
        res
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let data_callback = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        write_data(data, channels, &mut next_value)
    };
    let config = device.default_output_config().unwrap();
    let stream = device
        .build_output_stream(&config.config(), data_callback, err_fn)
        .unwrap();
    (Arc::new(stream), sample_rate, rx_req_sample, tx_sample)
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();

        let left: T = cpal::Sample::from::<f32>(&(sample.0 as f32));
        let right: T = cpal::Sample::from::<f32>(&(sample.1 as f32));

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}

pub fn init() -> (Arc<Stream>, f64, Receiver<bool>, Sender<(f64, f64)>) {
    let host = cpal::default_host();
    let mut device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&mut device, &config.into()),
        cpal::SampleFormat::I16 => run::<i16>(&mut device, &config.into()),
        cpal::SampleFormat::U16 => run::<u16>(&mut device, &config.into()),
    }
}
