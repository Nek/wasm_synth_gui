use cpal::BufferSize;
use cpal::SampleRate;
use cpal::StreamConfig;

use ringbuf::Consumer;
use ringbuf::Producer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;
use std::sync::Arc;

use std::sync::Mutex;

use ringbuf::StaticRb;

use crate::audio::AudioOutput;

#[allow(unused_imports)]
use cpal::traits::StreamTrait;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use glicol_synth::{operator::Mul, oscillator::SinOsc, AudioContext, AudioContextBuilder};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = " \
    export function unlockAudioContext(cb) { \
    const b = document.body; \
    const events = [\"click\", \"touchstart\", \"touchend\", \"mousedown\", \"keydown\"]; \
    events.forEach(e => b.addEventListener(e, unlock, false)); \
    function unlock() {cb(); clean(); console.log(\"!!!!!!!\")} \
    function clean() {events.forEach(e => b.removeEventListener(e, unlock));} \
}")]
extern "C" {
    fn unlockAudioContext(closure: &Closure<dyn FnMut()>);
}

const BUFFER_SIZE: usize = 256;
const SAMPLE_RATE: u32 = 44100;

type Event = ();

pub struct TemplateApp {
    audio_output_mtx: Arc<Mutex<AudioOutput>>,
    net_mtx: Arc<Mutex<AudioContext<1>>>,
    sample_producer_mtx: Arc<
        Mutex<
            Producer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>>,
        >,
    >,
    event_producer_mtx: Arc<Mutex<Producer<Event, Arc<SharedRb<Event, [MaybeUninit<Event>; 1]>>>>>,
    event_consumer_mtx: Arc<Mutex<Consumer<Event, Arc<SharedRb<Event, [MaybeUninit<Event>; 1]>>>>>,
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let audio_output_mtx: Arc<Mutex<AudioOutput>> =
            AudioOutput::new().expect("Can't create AudioOutput.");

        let audio_output_config_mtx = audio_output_mtx.clone();
        let mut audio_output_config = audio_output_config_mtx
            .lock()
            .expect("Can't lock AudioOutput.");

        let stream_config: &mut StreamConfig = &mut audio_output_config.supported_config.config();
        stream_config.buffer_size = BufferSize::Fixed(BUFFER_SIZE as u32);
        stream_config.sample_rate = SampleRate(SAMPLE_RATE);

        let samples_ringbuf = StaticRb::<(f64, f64), { BUFFER_SIZE }>::default();
        let (samples_producer, samples_consumer) = samples_ringbuf.split();

        let ready_audio_output_mtx: Arc<Mutex<AudioOutput>> = audio_output_config
            .setup::<BUFFER_SIZE>(stream_config, samples_consumer)
            .expect("Can't setup AudioOutput.");

        #[cfg(target_arch = "wasm32")]
        {
            let audio_output_mtx = ready_audio_output_mtx.clone();
            let f = move || {
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .play();
            };
            let cb = Closure::once(f);
            unlockAudioContext(&cb);
            cb.forget();
        }

        let net_mtx: Arc<Mutex<AudioContext<1>>> = Arc::new(Mutex::new(
            AudioContextBuilder::<1>::new()
                .sr(44100)
                .channels(2)
                .build(),
        ));
        let sample_producer_mtx = Arc::new(Mutex::new(samples_producer));

        let (event_producer, event_consumer) = StaticRb::<Event, 1>::default().split();

        TemplateApp {
            audio_output_mtx: ready_audio_output_mtx,
            sample_producer_mtx,
            net_mtx,
            event_producer_mtx: Arc::new(Mutex::new(event_producer)),
            event_consumer_mtx: Arc::new(Mutex::new(event_consumer)),
        }
    }
}

impl eframe::App for TemplateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Audio Panel");
            if ui.button("Start DSP").clicked() {
                let net_mtx = self.net_mtx.clone();
                let sample_producer_mtx = self.sample_producer_mtx.clone();
                let event_consumer_mtx = self.event_consumer_mtx.clone();
                thread::spawn(move || {
                    let mut sample_producer = sample_producer_mtx
                        .lock()
                        .expect("Can't lock sample_producer.");

                    let event_consumer = event_consumer_mtx
                        .lock()
                        .expect("Can't lock event consumer.");
                    loop {
                        if event_consumer.is_empty() {
                            while sample_producer.len() < BUFFER_SIZE {
                                let mut n = net_mtx.lock().expect("Can't lock Net64.");
                                let res = n.next_block();
                                // if res != (0.0, 0.0) {
                                sample_producer
                                    .push((res[0][0] as f64, res[1][0] as f64))
                                    .unwrap();
                                // }
                            }
                        }
                    }
                });
            }

            if ui.button("Start Synth 1").clicked() {
                let context_mtx = self.net_mtx.clone();

                let mut context = context_mtx.lock().expect("Can't lock AudioContext.");
                let node_a = context.add_mono_node(SinOsc::new().freq(440.0));
                let node_b = context.add_stereo_node(Mul::new(0.1));
                context.connect(node_a, node_b);
                let dest = context.destination;
                context.connect(node_b, dest);

                let audio_output_mtx = self.audio_output_mtx.clone();
                let mut audio_output = audio_output_mtx.lock().expect("Can't lock AudioOutput.");
                audio_output.play();
            };
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}
}
