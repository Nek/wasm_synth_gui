use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use ringbuf::Consumer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::{Receiver, SyncSender};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use std::sync::Arc;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;
use cpal::Stream;

fn run<'a, T>(
    device: &'a mut cpal::Device,
    config: &'a cpal::StreamConfig,
    mut sample_cons: Consumer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; 1]>>>,
    sample_req_tx: SyncSender<()>,
) -> (Arc<Stream>, f64, SyncSender<()>)
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let (sample_resp_tx, sample_resp_rx): (SyncSender<()>, Receiver<()>) = sync_channel(1);

    let data_callback = move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
        for frame in output.chunks_mut(channels) {
            sample_req_tx.send(()).unwrap_or_default();
            sample_resp_rx.recv().unwrap_or_default();
            let sample = sample_cons.pop().unwrap_or((0.0_f64, 0.0_f64));
            write_sample(frame, sample);
        }
    };
    let config = device.default_output_config().unwrap();
    let stream = device
        .build_output_stream(&config.config(), data_callback, err_fn)
        .unwrap();
    (Arc::new(stream), sample_rate, sample_resp_tx)
}

fn write_sample<T>(frame: &mut [T], sample: (f64, f64))
where
    T: cpal::Sample,
{
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

pub fn init(
    sample_cons: Consumer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; 1]>>>,
    sample_req_tx: SyncSender<()>,
) -> (Arc<Stream>, f64, SyncSender<()>) {
    let host = cpal::default_host();
    let mut device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => {
            run::<f32>(&mut device, &config.into(), sample_cons, sample_req_tx)
        }
        cpal::SampleFormat::I16 => {
            run::<i16>(&mut device, &config.into(), sample_cons, sample_req_tx)
        }
        cpal::SampleFormat::U16 => {
            run::<u16>(&mut device, &config.into(), sample_cons, sample_req_tx)
        }
    }
}
