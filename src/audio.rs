use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use ringbuf::Consumer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;
use std::sync::mpsc::{channel, Receiver, Sender, SyncSender};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread::JoinHandle;

use std::sync::Arc;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;

pub enum StreamAction {
    Play,
    Pause,
}

fn run<'a, T>(
    config: &'a cpal::StreamConfig,
    mut sample_cons: Consumer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; 1]>>>,
    sample_req_tx: SyncSender<()>,
) -> (Sender<StreamAction>, f64)
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let (stream_action_tx, stream_action_rx): (Sender<StreamAction>, Receiver<StreamAction>) =
        channel();
    let _thread: JoinHandle<_> = thread::spawn(move || {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("failed to find a default output device");

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let data_callback = move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in output.chunks_mut(channels) {
                let tx = sample_req_tx.clone();
                tx.send(()).unwrap_or_default();
                while sample_cons.len() < 1 {}
                write_sample(frame, sample_cons.pop().unwrap_or((0.0_f64, 0.0_f64)));
            }
        };
        let config = device.default_output_config().unwrap();
        let stream = device
            .build_output_stream(&config.config(), data_callback, err_fn)
            .unwrap();

        while let Ok(action) = stream_action_rx.recv() {
            match action {
                StreamAction::Play => stream.play().unwrap(),
                StreamAction::Pause => stream.pause().unwrap(),
            }
        }
    });
    (stream_action_tx, sample_rate)
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
) -> (Sender<StreamAction>, f64) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&config.into(), sample_cons, sample_req_tx),
        cpal::SampleFormat::I16 => run::<i16>(&config.into(), sample_cons, sample_req_tx),
        cpal::SampleFormat::U16 => run::<u16>(&config.into(), sample_cons, sample_req_tx),
    }
}
