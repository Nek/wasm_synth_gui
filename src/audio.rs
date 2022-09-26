use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use cpal::Device;

use cpal::Stream;
use cpal::StreamConfig;
use cpal::SupportedStreamConfig;

use ringbuf::Consumer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;

use std::sync::Arc;
use std::sync::Mutex;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;

pub enum AudioOutputState {
    Init,
    Ready,
    Playing,
    Paused,
}

pub struct AudioOutput {
    pub state: AudioOutputState,
    pub supported_config: SupportedStreamConfig,
    pub config: Option<StreamConfig>,
    stream: Option<Arc<Stream>>,
}

impl AudioOutput {
    pub fn new() -> Option<Arc<Mutex<Self>>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let supported_config = device
            .default_output_config()
            .expect("Can't get default output config.");
        Some(Arc::new(Mutex::new(AudioOutput {
            state: AudioOutputState::Init,
            supported_config,
            config: None,
            stream: None,
        })))
    }
    pub fn play(&mut self) {
        match self.state {
            AudioOutputState::Init => (),
            _ => {
                self.stream
                    .as_mut()
                    .expect("Audio Stream is missing.")
                    .play()
                    .expect("Can't play audio Stream.");
                self.state = AudioOutputState::Playing;
            }
        }
    }
    pub fn pause(&mut self) {
        match self.state {
            AudioOutputState::Playing => {
                self.stream
                    .as_mut()
                    .expect("Audio Stream is missing.")
                    .pause()
                    .expect("Can't pause audio Stream.");
                self.state = AudioOutputState::Paused;
            }
            _ => (),
        }
    }
    pub fn setup<const BUFFER_SIZE: usize>(
        &mut self,
        config: &mut StreamConfig,
        sample_consumer: Consumer<
            (f64, f64),
            Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>,
        >,
    ) -> Option<Arc<Mutex<Self>>> {
        match self.state {
            AudioOutputState::Init => {
                let host = cpal::default_host();
                let device = host
                    .default_output_device()
                    .expect("failed to find a default output device");

                let sample_format = self.supported_config.sample_format();

                let stream = match sample_format {
                    cpal::SampleFormat::F32 => {
                        run::<f32, BUFFER_SIZE>(device, &config.clone(), sample_consumer)
                    }
                    cpal::SampleFormat::I16 => {
                        run::<i16, BUFFER_SIZE>(device, &config.clone(), sample_consumer)
                    }
                    cpal::SampleFormat::U16 => {
                        run::<u16, BUFFER_SIZE>(device, &config.clone(), sample_consumer)
                    }
                };

                self.state = AudioOutputState::Ready;
                self.config = Some(config.clone());
                self.stream = Some(Arc::new(stream));

                Some(Arc::new(Mutex::new(AudioOutput {
                    state: AudioOutputState::Ready,
                    supported_config: self.supported_config.clone(),
                    config: self.config.clone(),
                    stream: self.stream.clone(),
                })))
            }
            _ => None,
        }
    }
}

fn run<'a, T, const BUFFER_SIZE: usize>(
    device: Device,
    config: &'a cpal::StreamConfig,
    mut sample_cons: Consumer<
        (f64, f64),
        Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>,
    >,
) -> Stream
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
        .expect("Can't build output Stream");

    #[cfg(not(target_arch = "wasm32"))]
    stream.pause().expect("Can't pause audio Stream.");

    stream
}
