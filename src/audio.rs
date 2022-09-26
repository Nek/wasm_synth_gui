use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::BufferSize;
use cpal::Device;
use cpal::SampleRate;
use cpal::Stream;
use cpal::StreamConfig;

use ringbuf::Consumer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;
use std::sync::Arc;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;

fn run<'a, T, const BUFFER_SIZE: usize>(
    device: Device,
    config: &'a cpal::StreamConfig,
    mut sample_cons: Consumer<
        (f64, f64),
        Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>,
    >,
) -> Arc<Stream>
where
    T: cpal::Sample + Send,
{
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in output.chunks_mut(channels) {
                    if !sample_cons.is_empty() {
                        if let Some(current_sample) = sample_cons.pop() {
                            let left: T = cpal::Sample::from::<f32>(&(current_sample.0 as f32));
                            let right: T = cpal::Sample::from::<f32>(&(current_sample.1 as f32));

                            for (channel, sample) in frame.iter_mut().enumerate() {
                                if channel & 1 == 0 {
                                    *sample = left;
                                } else {
                                    *sample = right;
                                }
                            }
                        }
                    }
                }
            },
            err_fn,
        )
        .unwrap();
    println!("stream is built");
    stream.play().unwrap();

    Arc::new(stream)
}

pub fn init<const BUFFER_SIZE: usize>(
    sample_cons: Consumer<
        (f64, f64),
        Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>,
    >,
    sample_rate: u32,
) -> Arc<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    let sample_format = config.sample_format();
    let conf: &mut StreamConfig = &mut config.into();
    conf.buffer_size = BufferSize::Fixed(BUFFER_SIZE as u32);
    conf.sample_rate = SampleRate(sample_rate);

    match sample_format {
        cpal::SampleFormat::F32 => run::<f32, BUFFER_SIZE>(device, &conf, sample_cons),
        cpal::SampleFormat::I16 => run::<i16, BUFFER_SIZE>(device, &conf, sample_cons),
        cpal::SampleFormat::U16 => run::<u16, BUFFER_SIZE>(device, &conf, sample_cons),
    }
}
