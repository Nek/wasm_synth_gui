use cpal::BufferSize;
use cpal::SampleRate;
use cpal::StreamConfig;

use fundsp::hacker::*;
use fundsp::prelude::Net64;
use ringbuf::Producer;
use ringbuf::SharedRb;

use std::mem::MaybeUninit;
use std::sync::Arc;

use std::sync::mpsc::channel;
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use ringbuf::StaticRb;

use crate::audio::AudioOutput;
use crate::audio::AudioOutputState;
#[allow(unused_imports)]
use cpal::traits::StreamTrait;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
// #[cfg(target_arch = "wasm32")]
// macro_rules! console_log {
//     // Note that this is using the `log` function imported above during
//     // `bare_bones`
//     ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
// }

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = " \
    export function unlockAudioContext(cb) { \
    const b = document.body; \
    const events = [\"click\", \"touchstart\", \"touchend\", \"mousedown\", \"keydown\"]; \
    events.forEach(e => b.addEventListener(e, unlock, false)); \
    function unlock() {cb(); clean();} \
    function clean() {events.forEach(e => b.removeEventListener(e, unlock));} \
}")]
extern "C" {
    fn unlockAudioContext(closure: &Closure<dyn FnMut()>);
}

const BUFFER_SIZE: usize = 256;
const SAMPLE_RATE: u32 = 44100;

pub struct TemplateApp {
    audio_output_mtx: Arc<Mutex<AudioOutput>>,
    net_mtx: Arc<Mutex<Net64>>,
    sample_producer_mtx: Arc<
        Mutex<
            Producer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>>,
        >,
    >,
}

impl TemplateApp {
    /// Called once before the first frame.
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
        let (producer, consumer) = samples_ringbuf.split();

        let ready_audio_output_mtx: Arc<Mutex<AudioOutput>> = audio_output_config
            .setup::<BUFFER_SIZE>(stream_config, consumer)
            .expect("Can't setup AudioOutput.");

        #[cfg(target_arch = "wasm32")]
        {
            let audio_output_mtx = ready_audio_output_mtx.clone();
            let f = move || {
                println!("let's play and pause");
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .play();
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .pause();
                println!("Unlock web audio done");
            };
            let cb = Closure::once(f);
            unlockAudioContext(&cb);
            println!("Start unlock web audio");
            cb.forget();
        }

        let net_mtx: Arc<Mutex<Net64>> = Arc::new(Mutex::new(Net64::new(0, 1)));
        let sample_producer_mtx = Arc::new(Mutex::new(producer));

        TemplateApp {
            audio_output_mtx: ready_audio_output_mtx,
            sample_producer_mtx,
            net_mtx,
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
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

            if ui.button("Start DSP loop.").clicked() {
                let net_mtx = self.net_mtx.clone();
                let producer_mtx = self.sample_producer_mtx.clone();
                let _update_net_thread = thread::spawn(move || {
                    let mut net = net_mtx.lock().expect("Can't lock Net64.");

                    println!("lock net");

                    println!("len() {}", producer_mtx.lock().unwrap().len());

                    let mut producer = producer_mtx.lock().expect("Can't lock sample_producer.");
                    loop {
                        while producer.len() < BUFFER_SIZE {
                            let res = net.get_stereo();
                            if res != (0.0, 0.0) {
                                producer.push(res).unwrap();
                            }
                        }
                    }
                });
            }

            if ui.button("Play").clicked() {
                let audio_output_mtx = self.audio_output_mtx.clone();
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .play();
            }

            if ui.button("Pause").clicked() {
                let audio_output_mtx = self.audio_output_mtx.clone();
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .pause();
            }

            if ui.button("Setup synth 1").clicked() {
                let net_mtx = self.net_mtx.clone();
                let _update_net_thread = thread::spawn(move || {
                    let mut net = net_mtx.lock().expect("Can't lock Net64.");
                    let dc_id = net.push(Box::new(dc(220.0)));
                    let sine_id = net.push(Box::new(sine()));
                    // Connect nodes.
                    net.pipe(dc_id, sine_id);
                    net.pipe_output(sine_id);
                    net.reset(Some(SAMPLE_RATE.into()));
                });
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
