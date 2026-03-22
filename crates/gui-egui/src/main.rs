use eframe::egui;
use estima_core::audio::{AudioState as JackAudioState, JackEngine, PluginChain};
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result {
    env_logger::init();

    let chain = Arc::new(Mutex::new(PluginChain::new().unwrap()));

    let audio_state = Arc::new(Mutex::new(JackAudioState {
        process_fn: Box::new(|_input: &[f32], _output: &mut [f32], _nframes: usize| {}),
    }));

    let jack_engine = JackEngine::new("estima-egui", audio_state.clone())
        .expect("Failed to create JACK client. Is JACK running?");

    log::info!("JACK client '{}' created", jack_engine.client_name());
    let sample_rate = jack_engine.sample_rate() as f64;
    log::info!(
        "Sample rate: {}, Buffer size: {}",
        sample_rate,
        jack_engine.buffer_size()
    );

    {
        let chain_clone = chain.clone();
        let mut state = audio_state.lock().unwrap();
        state.process_fn = Box::new(move |input: &[f32], output: &mut [f32], nframes: usize| {
            if let Ok(mut c) = chain_clone.lock() {
                c.process(input, output, nframes);
            }
        });
    }

    let _ = jack_engine;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Estima - AI Audio Effects",
        options,
        Box::new(|cc| Ok(Box::new(EstimaApp::new(cc, chain, sample_rate)))),
    )
}

struct EstimaApp {
    chain: Arc<Mutex<PluginChain>>,
    sample_rate: f64,
    filter: String,
    plugins: Vec<(String, String)>,       // name, uri
    chain_plugins: Vec<(String, String)>, // id, name
    status: String,
    pending_action: Option<AppAction>,
}

#[derive(Clone)]
enum AppAction {
    LoadPlugin(String, String), // uri, name
    RemovePlugin(String),       // id
    ClearAll,
    ToggleBypass,
}

impl EstimaApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        chain: Arc<Mutex<PluginChain>>,
        sample_rate: f64,
    ) -> Self {
        let plugins: Vec<_> = {
            let c = chain.lock().unwrap();
            c.list_available_plugins()
                .iter()
                .take(50)
                .map(|p| (p.name.clone(), p.uri.clone()))
                .collect()
        };

        let mut app = Self {
            chain: chain.clone(),
            sample_rate,
            filter: String::new(),
            plugins,
            chain_plugins: Vec::new(),
            status: "Ready".to_string(),
            pending_action: None,
        };
        app.refresh_chain();
        app
    }

    fn refresh_chain(&mut self) {
        let c = self.chain.lock().unwrap();
        self.chain_plugins = c
            .get_active_plugins()
            .iter()
            .map(|p| (p.id.clone(), p.info.name.clone()))
            .collect();
    }

    fn execute_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            match action {
                AppAction::LoadPlugin(uri, name) => {
                    let mut c = self.chain.lock().unwrap();
                    if let Ok((_, _)) = c.load_plugin(&uri, self.sample_rate) {
                        self.status = format!("Loaded: {}", name);
                        drop(c);
                        self.refresh_chain();
                    }
                }
                AppAction::RemovePlugin(id) => {
                    let mut c = self.chain.lock().unwrap();
                    let _ = c.remove_plugin(&id);
                    drop(c);
                    self.refresh_chain();
                }
                AppAction::ClearAll => {
                    let mut c = self.chain.lock().unwrap();
                    c.clear();
                    drop(c);
                    self.refresh_chain();
                    self.status = "Cleared".to_string();
                }
                AppAction::ToggleBypass => {
                    let mut c = self.chain.lock().unwrap();
                    c.toggle_bypass();
                }
            }
        }
    }
}

impl eframe::App for EstimaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Execute any pending actions from previous frame
        self.execute_pending_action();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Estima - AI Audio Effects");
            ui.label(&self.status);
            ui.separator();

            ui.columns(2, |columns| {
                // Left: Available plugins
                columns[0].group(|ui| {
                    ui.heading("Available Plugins");
                    ui.text_edit_singleline(&mut self.filter);

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for (name, uri) in &self.plugins {
                                if name.to_lowercase().contains(&self.filter.to_lowercase()) {
                                    ui.horizontal(|ui| {
                                        ui.label(name);
                                        if ui.button("Load").clicked() {
                                            self.pending_action = Some(AppAction::LoadPlugin(
                                                uri.clone(),
                                                name.clone(),
                                            ));
                                        }
                                    });
                                }
                            }
                        });
                });

                // Right: Effect chain
                columns[1].group(|ui| {
                    ui.heading("Effect Chain");

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (id, name) in &self.chain_plugins {
                                ui.horizontal(|ui| {
                                    ui.label(format!("• {}", name));
                                    if ui.button("X").clicked() {
                                        self.pending_action =
                                            Some(AppAction::RemovePlugin(id.clone()));
                                    }
                                });
                            }
                        });

                    ui.separator();

                    if ui.button("Clear All").clicked() {
                        self.pending_action = Some(AppAction::ClearAll);
                    }

                    let bypass = {
                        let c = self.chain.lock().unwrap();
                        c.bypass()
                    };

                    if ui
                        .button(if bypass { "Bypass: ON" } else { "Bypass: OFF" })
                        .clicked()
                    {
                        self.pending_action = Some(AppAction::ToggleBypass);
                    }
                });
            });
        });
    }
}
