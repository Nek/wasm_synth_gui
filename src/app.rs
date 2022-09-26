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
    sample_producer: Arc<
        Mutex<
            Producer<(f64, f64), Arc<SharedRb<(f64, f64), [MaybeUninit<(f64, f64)>; BUFFER_SIZE]>>>,
        >,
    >,
}

enum NetCommand {
    Hold,
    Resume,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let audio_output_mtx: Arc<Mutex<AudioOutput>> =
            AudioOutput::new().expect("Can't create AudioOutput.");

        let binding = audio_output_mtx.clone();
        let mut audio_output = binding.lock().expect("Can't lock AudioOutput.");

        let stream_config: &mut StreamConfig = &mut audio_output.supported_config.config();
        stream_config.buffer_size = BufferSize::Fixed(BUFFER_SIZE as u32);
        stream_config.sample_rate = SampleRate(SAMPLE_RATE);

        let samples_ringbuf = StaticRb::<(f64, f64), { BUFFER_SIZE }>::default();
        let (producer, consumer) = samples_ringbuf.split();

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
        let producer_mtx = Arc::new(Mutex::new(producer));

        let compute_net_mtx = net_mtx.clone();
        let compute_producer_mtx = producer_mtx.clone();
        let (tx, rx) = channel::<NetCommand>();

        let _compute_net_send_samples_thread = thread::spawn(move || {
            let mut net = compute_net_mtx.lock().expect("Can't lock Net64.");
            let mut producer = compute_producer_mtx
                .lock()
                .expect("Can't lock sample_producer.");

            loop {
                while producer.len() < BUFFER_SIZE {
                    let res = net.get_stereo();
                    // if res != (0.0, 0.0) {
                    producer.push(res).unwrap();
                    // }
                }
            }
        });

        TemplateApp {
            audio_output_mtx: ready_audio_output_mtx,
            net_mtx,
            sample_producer: producer_mtx,
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

            if ui.add_enabled(true, egui::Button::new("Beep")).clicked() {
                let net_mtx = self.net_mtx.clone();
                // let producer_mtx = self.sample_producer.clone();
                let producer_mtx = self.sample_producer.clone();
                let _update_net_thread = thread::spawn(move || {
                    println!("lock net");

                    let mut net = net_mtx.lock().expect("Can't lock Net64.");
                    // let c = dc(220.0) >> square();

                    // let pulse_id = net.push(Box::new(c));

                    let dc_id = net.push(Box::new(dc(220.0)));
                    let sine_id = net.push(Box::new(sine()));
                    // Connect nodes.
                    net.pipe(dc_id, sine_id);
                    net.pipe_output(sine_id);

                    // net.pipe_output(pulse_id);

                    net.reset(Some(SAMPLE_RATE.into()));

                    println!("len() {}", producer_mtx.lock().unwrap().len());

                    // let mut producer = producer_mtx.lock().expect("Can't lock sample_producer.");
                    // loop {
                    //     while producer.len() < BUFFER_SIZE {
                    //         let res = net.get_stereo();
                    //         if res != (0.0, 0.0) {
                    //             producer.push(res).unwrap();
                    //         }
                    //     }
                    // }

                    // let cycle = (BUFFER_SIZE as u32) * 60000 / SAMPLE_RATE;

                    // loop {
                    //     while prod.len() < BUFFER_SIZE {
                    //         let res = net.get_stereo();
                    //         prod.push(res).unwrap();
                    //     }
                    // }
                    drop(net);
                });
                let audio_output_mtx = self.audio_output_mtx.clone();
                audio_output_mtx
                    .lock()
                    .expect("Can't lock AudioOutput.")
                    .play();
                drop(audio_output_mtx);
                // let mut audio_output = self
                //     .audio_output_mtx
                //     .lock()
                //     .expect("Can't pull AudioOutput out of Mutex.");
                // println!(
                //     "{}",
                //     match audio_output.state {
                //         AudioOutputState::Init => "AudioOutputState::Init",
                //         AudioOutputState::Ready => "AudioOutputState::Ready",
                //         AudioOutputState::Playing => "AudioOutputState::Playing",
                //         AudioOutputState::Paused => "AudioOutputState::Paused",
                //     }
                // );
                // audio_output.play();
                // match audio_output.state {
                //     AudioOutputState::Init => (),
                //     AudioOutputState::Ready => audio_output.play(),
                //     AudioOutputState::Playing => audio_output.pause(),
                //     AudioOutputState::Paused => audio_output.play(),
                // }
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
