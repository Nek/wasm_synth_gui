/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct TemplateApp<'a> {
    stream: Stream,
    net: &'a Box<Net64>,
}

use crate::audio;
#[allow(unused_imports)]
use cpal::traits::StreamTrait;
use cpal::Stream;
use fundsp::hacker::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::{wasm_bindgen, Closure};

impl TemplateApp<'_> {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (net, stream) = audio::start();
        TemplateApp { stream, net }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = " \
    export function unlockAudioContext(cb) { \
    const b = document.body; \
    const events = [\"touchstart\", \"touchend\", \"mousedown\", \"keydown\"]; \
    events.forEach(e => b.addEventListener(e, unlock, false)); \
    function unlock() {cb(); clean();} \
    function clean() {events.forEach(e => b.removeEventListener(e, unlock));} \
}")]
extern "C" {
    fn unlockAudioContext(closure: &Closure<dyn FnMut()>);
}

impl eframe::App for TemplateApp<'_> {
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
                // Add nodes, obtaining their IDs.
                let net = &mut self.net;
                let dc_id = net.push(Box::new(dc(220.0)));
                let sine_id = net.push(Box::new(sine()));
                // Connect nodes.
                net.pipe(dc_id, sine_id);
                net.pipe_output(sine_id);
                // #[cfg(target_arch = "wasm32")]
                // {
                //     let f = || {
                //         if let Some(stream) = stream {
                //             stream.play().unwrap();
                //         };
                //     };
                //     // let bx: Box<dyn Fn()> = Box::new(f);
                //     let cb = Closure::once(f);
                //     unlockAudioContext(&cb);
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
