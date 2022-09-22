use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;
use cpal::Stream;

use fundsp::prelude::*;

fn run<'a, T>(device: &'a mut cpal::Device, config: &'a cpal::StreamConfig) -> Stream
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut net: Net64 = Net64::new(0, 1);

    net.reset(Some(sample_rate));

    let dc_id = net.push(Box::new(dc(220.0)));
    let sine_id = net.push(Box::new(sine()));
    // Connect nodes.
    net.pipe(dc_id, sine_id);
    net.pipe_output(sine_id);

    let mut next_value = move || net.get_stereo();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let data_callback = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        write_data(data, channels, &mut next_value)
    };
    let config = device.default_output_config().unwrap();
    let stream = device
        .build_output_stream(&config.config(), data_callback, err_fn)
        .unwrap();
    stream
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

pub fn init() -> Stream {
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
