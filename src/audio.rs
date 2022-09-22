use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::Data;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;
use cpal::Stream;

use fundsp::prelude::AudioUnit64;
use fundsp::prelude::Net64;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub struct ClickListener {
    // cb: Option<&'static mut Closure<dyn FnMut() -> () + 'static>>,
}

#[cfg(target_arch = "wasm32")]
impl ClickListener {
    pub fn new() -> ClickListener {
        // Self { cb: None }
        Self {}
    }
}
#[cfg(target_arch = "wasm32")]
trait AddClickCb<'a> {
    fn add(&'a self, cb: Closure<dyn FnMut() -> () + 'static>);
}

#[cfg(target_arch = "wasm32")]
impl<'a> AddClickCb<'a> for ClickListener {
    fn add(&'a self, cb: Closure<dyn FnMut() -> () + 'static>) {
        let window = web_sys::window().expect("no global `window` exists");
        let document: web_sys::Document =
            window.document().expect("should have a document on window");

        document
            .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .unwrap();

        let remove_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            let window = web_sys::window().expect("no global `window` exists");
            let document: web_sys::Document =
                window.document().expect("should have a document on window");
            document
                .remove_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
                .expect("can't remove \"click\" callback");
        }) as Box<dyn Fn()>);

        document
            .add_event_listener_with_callback("click", remove_cb.as_ref().unchecked_ref())
            .unwrap();

        remove_cb.forget();
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::Closure;

#[cfg(target_arch = "wasm32")]
fn resume<'a>(cls: Closure<dyn FnMut() + 'static>) {
    ClickListener::new().add(cls);
}

fn run<'a, T>(
    device: &'a mut cpal::Device,
    config: &'a cpal::StreamConfig,
) -> (&'a Box<Net64>, Stream)
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut graph: Box<Net64> = Box::new(Net64::new(0, 1));

    graph.reset(Some(sample_rate));

    let next_value: &mut dyn FnMut() -> (f64, f64) = &mut || graph.get_stereo();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let data_callback =
        |data: &mut Data, _: &cpal::OutputCallbackInfo| write_data(data, channels, &mut next_value);
    let config = device.default_output_config().unwrap();
    let stream = device
        .build_output_stream_raw(
            &config.config(),
            config.sample_format(),
            data_callback,
            err_fn,
        )
        .unwrap();
    // let stream = device
    //     .build_output_stream(config, data_callback, err_fn)
    //     .unwrap();
    (&graph, stream)
}

fn write_data<T>(output: &mut Data, channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: cpal::Sample,
{
    for frame in output.as_slice_mut() {
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

fn run_audio() -> (&'static Box<Net64>, Stream) {
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
pub fn start() -> (&'static Box<Net64>, Stream) {
    run_audio()
}
