/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct TemplateApp {
    audio_output_mtx: Arc<Mutex<AudioOutput>>,
    // stream: Arc<Stream>,
    // rx_req_sample: Receiver<bool>,
    // tx_sample: Sender<(f64, f64)>,
    net_mtx: Arc<Mutex<Net64>>,
    // is_playing: bool,
}

use cpal::BufferSize;
use cpal::SampleRate;
use cpal::StreamConfig;

use fundsp::hacker::*;
use fundsp::prelude::Net64;

use std::sync::Arc;

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

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let audio_output_mtx: Arc<Mutex<AudioOutput>> =
            AudioOutput::new().expect("Can't init AudioOutput.");

        let binding = audio_output_mtx.clone();
        let mut audio_output = binding
            .lock()
            .expect("Can't pull AudioOutput out of Mutex.");

        let stream_config: &mut StreamConfig = &mut audio_output.supported_config.config();
        stream_config.buffer_size = BufferSize::Fixed(BUFFER_SIZE as u32);
        stream_config.sample_rate = SampleRate(44100);

        const BUFFER_SIZE: usize = 256;
        const SAMPLE_RATE: u32 = 44100;
        let samples_ringbuf = StaticRb::<(f64, f64), { BUFFER_SIZE }>::default();
        let (mut producer, consumer) = samples_ringbuf.split();

        let ready_audio_output_mtx: Arc<Mutex<AudioOutput>> = audio_output
            .setup::<BUFFER_SIZE>(stream_config, consumer)
            .expect("Can't setup AudioOutput.");

        #[cfg(target_arch = "wasm32")]
        {
            let audio_output_mtx = ready_audio_output_mtx.clone();
            let f = move || {
                println!("let's play and pause");
                audio_output_mtx
                    .lock()
                    .expect("Can't pull AudioOutput out of Mutex.")
                    .play();
                audio_output_mtx
                    .lock()
                    .expect("Can't pull AudioOutput out of Mutex.")
                    .pause();
                println!("unlock done");
            };
            let cb = Closure::once(f);
            unlockAudioContext(&cb);
            println!("start unlock");
            cb.forget();
        }

        let net_mtx: Arc<Mutex<Net64>> = Arc::new(Mutex::new(Net64::new(0, 1)));

        let net_mtx2 = net_mtx.clone();
        let _net_thread = thread::spawn(move || {
            if let Ok(mut net) = net_mtx2.lock() {
                // let dc_id = net.push(Box::new(dc(220.0)));
                // let sine_id = net.push(Box::new(sine()));
                // net.pipe(dc_id, sine_id);
                // net.pipe_output(sine_id);
                // net.reset(Some(SAMPLE_RATE.into()));

                // let cycle = (BUFFER_SIZE as u32) * 60000 / SAMPLE_RATE;

                loop {
                    while producer.len() < BUFFER_SIZE {
                        let res = net.get_stereo();
                        producer.push(res).unwrap();
                    }
                }
            }
        });

        let net_mtx3 = net_mtx.clone();
        let _net_thread2 = thread::spawn(move || {
            if let Ok(mut net) = net_mtx3.lock() {
                let dc_id = net.push(Box::new(dc(220.0)));
                let sine_id = net.push(Box::new(sine()));
                net.pipe(dc_id, sine_id);
                net.pipe_output(sine_id);
                net.reset(Some(SAMPLE_RATE.into()));

                // let cycle = (BUFFER_SIZE as u32) * 60000 / SAMPLE_RATE;

                // loop {
                //     while prod.len() < BUFFER_SIZE {
                //         let res = net.get_stereo();
                //         prod.push(res).unwrap();
                //     }
                // }
            }
        });

        TemplateApp {
            audio_output_mtx: ready_audio_output_mtx,
            net_mtx,
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // let Self { stream } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

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
            if ui.add_enabled(true, egui::Button::new("Beep")).clicked() {
                let mut audio_output = self
                    .audio_output_mtx
                    .lock()
                    .expect("Can't pull AudioOutput out of Mutex.");
                println!(
                    "{}",
                    match audio_output.state {
                        AudioOutputState::Init => "AudioOutputState::Init",
                        AudioOutputState::Ready => "AudioOutputState::Ready",
                        AudioOutputState::Playing => "AudioOutputState::Playing",
                        AudioOutputState::Paused => "AudioOutputState::Paused",
                    }
                );
                match audio_output.state {
                    AudioOutputState::Init => (),
                    AudioOutputState::Ready => audio_output.play(),
                    AudioOutputState::Playing => audio_output.pause(),
                    AudioOutputState::Paused => audio_output.play(),
                }
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
