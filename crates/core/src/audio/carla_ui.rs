use crate::ffi::carla::{
    carla_add_plugin, carla_embed_custom_ui, carla_engine_close, carla_engine_idle,
    carla_engine_init, carla_get_current_plugin_count, carla_remove_plugin,
    carla_set_custom_ui_title, carla_show_custom_ui,
    CarlaBackendEngineMode, CarlaEngineHandle, CarlaPluginId, CarlaPluginType,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, Mutex};
use gtk::prelude::*;
use glib::Propagation;

struct UIInstance {
    plugin_id: CarlaPluginId,
    window: gtk::Window,
}

pub struct CarlaUIManager {
    engine: CarlaEngineHandle,
    instances: Arc<Mutex<HashMap<String, UIInstance>>>,
    chain: Arc<Mutex<crate::audio::PluginChain>>,
}

unsafe impl Send for CarlaUIManager {}

impl CarlaUIManager {
    pub fn new(chain: Arc<Mutex<crate::audio::PluginChain>>) -> Result<Self> {
        unsafe {
            let driver = CString::new("JACK").unwrap();
            let client_name = CString::new("estima-ui").unwrap();

            let engine = carla_engine_init(driver.as_ptr(), client_name.as_ptr());

            if engine.is_null() {
                return Err(anyhow!("Failed to initialize Carla engine (is JACK running?)"));
            }

            log::info!("Carla engine initialized");

            Ok(Self {
                engine,
                instances: Arc::new(Mutex::new(HashMap::new())),
                chain,
            })
        }
    }

    pub fn open_ui(&mut self, plugin_id: &str) -> Result<()> {
        {
            let instances = self.instances.lock().unwrap();
            if instances.contains_key(plugin_id) {
                log::info!("UI already open for plugin {}", plugin_id);
                drop(instances);
                let instances = self.instances.lock().unwrap();
                if let Some(instance) = instances.get(plugin_id) {
                    instance.window.present();
                }
                return Ok(());
            }
        }

        let (plugin_uri, plugin_name) = {
            let chain = self.chain.lock().map_err(|e| anyhow!("{}", e))?;
            let uri = chain.get_plugin_uri_by_id(plugin_id)?;
            let name = chain.get_plugin_name_by_id(plugin_id)?;
            (uri, name)
        };

        log::info!(
            "Opening UI for plugin {} via Carla: {}",
            plugin_id,
            plugin_uri
        );

        self.open_carla_ui(plugin_id, &plugin_uri, &plugin_name)
    }

    fn open_carla_ui(&mut self, plugin_id: &str, plugin_uri: &str, plugin_name: &str) -> Result<()> {
        unsafe {
            let filename = CString::new("").unwrap();
            let name = CString::new(plugin_name).unwrap();
            let label = CString::new(plugin_uri).unwrap();

            let result = carla_add_plugin(
                self.engine,
                CarlaBackendEngineMode::EngineModeSingleClient,
                CarlaPluginType::PluginTypeLV2,
                filename.as_ptr(),
                name.as_ptr(),
                label.as_ptr(),
                0,
                std::ptr::null(),
                0,
            );

            if result != 0 {
                return Err(anyhow!("Failed to add plugin to Carla: {}", result));
            }

            let carla_plugin_id = carla_get_current_plugin_count(self.engine) - 1;
            log::info!("Plugin added to Carla with ID: {}", carla_plugin_id);

            let window = gtk::Window::new(gtk::WindowType::Toplevel);
            window.set_title(&format!("Plugin: {}", plugin_name));
            window.set_default_size(400, 300);

            let socket = gtk::Socket::new();
            window.add(&socket);

            window.show_all();

            while gtk::events_pending() {
                gtk::main_iteration();
            }

            let socket_xid = socket.id() as u64;
            log::info!("GtkSocket XID: {}", socket_xid);

            let title = CString::new(plugin_name).unwrap();
            carla_set_custom_ui_title(self.engine, carla_plugin_id, title.as_ptr());

            let embed_result = carla_embed_custom_ui(self.engine, carla_plugin_id, socket_xid);
            log::info!("Embed UI result: {}", embed_result);

            if !embed_result {
                carla_remove_plugin(self.engine, carla_plugin_id);
                return Err(anyhow!("Failed to embed plugin UI"));
            }

            let show_result = carla_show_custom_ui(self.engine, carla_plugin_id, true);
            log::info!("Show UI result: {}", show_result);

            let instances_clone = self.instances.clone();
            let plugin_id_clone = plugin_id.to_string();
            let engine = self.engine;

            window.connect_delete_event(move |_w, _| {
                log::info!("UI window closed for {}", plugin_id_clone);
                if let Ok(mut insts) = instances_clone.lock() {
                    if let Some(inst) = insts.remove(&plugin_id_clone) {
                        carla_show_custom_ui(engine, inst.plugin_id, false);
                        carla_remove_plugin(engine, inst.plugin_id);
                    }
                }
                Propagation::Proceed
            });

            {
                let mut instances = self.instances.lock().unwrap();
                instances.insert(
                    plugin_id.to_string(),
                    UIInstance {
                        plugin_id: carla_plugin_id,
                        window,
                    },
                );
            }

            Ok(())
        }
    }

    pub fn close_ui(&mut self, plugin_id: &str) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(instance) = instances.remove(plugin_id) {
            instance.window.close();
            unsafe {
                carla_show_custom_ui(self.engine, instance.plugin_id, false);
                carla_remove_plugin(self.engine, instance.plugin_id);
            }
            log::info!("Closed UI for plugin {}", plugin_id);
        }
        Ok(())
    }

    pub fn close_all(&mut self) {
        let mut instances = self.instances.lock().unwrap();
        for (plugin_id, instance) in instances.drain() {
            instance.window.close();
            unsafe {
                carla_show_custom_ui(self.engine, instance.plugin_id, false);
                carla_remove_plugin(self.engine, instance.plugin_id);
            }
            log::info!("Closed UI for plugin {}", plugin_id);
        }
    }

    pub fn is_ui_open(&self, plugin_id: &str) -> bool {
        let instances = self.instances.lock().unwrap();
        instances.contains_key(plugin_id)
    }

    pub fn idle(&self) {
        unsafe {
            carla_engine_idle(self.engine);
        }
    }
}

impl Drop for CarlaUIManager {
    fn drop(&mut self) {
        self.close_all();
        unsafe {
            carla_engine_close(self.engine);
        }
        log::info!("Carla engine closed");
    }
}
