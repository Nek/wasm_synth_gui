/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct TemplateApp {
    stream_action_tx: Sender<StreamAction>,
    // rx_req_sample: Receiver<bool>,
    // tx_sample: Sender<(f64, f64)>,
    net_mtx: Arc<Mutex<Net64>>,
    is_playing: bool,
}

use fundsp::hacker::*;
use fundsp::prelude::Net64;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::Sender;

use std::sync::Arc;

use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use ringbuf::StaticRb;

use crate::audio;
use crate::audio::StreamAction;
#[allow(unused_imports)]
use cpal::traits::StreamTrait;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        const RB_SIZE: usize = 1;
        let sample_buf = StaticRb::<(f64, f64), RB_SIZE>::default();
        let (mut prod, cons) = sample_buf.split();

        let (sample_req_tx, sample_req_rx) = sync_channel(1);

        let (stream_action_tx, sample_rate): (Sender<StreamAction>, f64) =
            audio::init(cons, sample_req_tx);

        let net_mtx: Arc<Mutex<Net64>> = Arc::new(Mutex::new(Net64::new(0, 1)));

        let net_mtx2 = net_mtx.clone();

        let _net_thread = thread::spawn(move || {
            if let Ok(mut net) = net_mtx2.lock() {
                let dc_id = net.push(Box::new(dc(220.0)));
                let sine_id = net.push(Box::new(sine()));
                net.pipe(dc_id, sine_id);
                net.pipe_output(sine_id);

                net.reset(Some(sample_rate));

                loop {
                    sample_req_rx.recv().unwrap();
                    let res = net.get_stereo();
                    prod.push(res).unwrap();
                }
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        stream_action_tx.send(StreamAction::Pause).unwrap();

        #[cfg(target_arch = "wasm32")]
        {
            let s = stream_action_tx.clone();
            let f = move || {
                s.send(StreamAction::Pause).unwrap();
            };
            let cb = Closure::once(f);
            unlockAudioContext(&cb);
            cb.forget()
        }
        TemplateApp {
            stream_action_tx,
            is_playing: false,
            net_mtx,
            // net,
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
                if self.is_playing {
                    self.stream_action_tx.send(StreamAction::Pause).unwrap();
                } else {
                    self.stream_action_tx.send(StreamAction::Play).unwrap();
                }
                self.is_playing = !self.is_playing;
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
