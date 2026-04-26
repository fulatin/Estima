use crate::ffi::lv2_ui::{
    LV2_Feature, LV2_URID_Map, LV2_URID_Unmap, LV2_URID__map, LV2_URID__unmap,
    LV2_UI__idleInterface, LV2_UI__parent, UridMap, UridMapPtr,
};
use crate::ffi::suil::{self, SUIL_ARG_NONE, SuilController, SuilHost, SuilInstance};
use anyhow::{anyhow, Result};
use std::ffi::{c_void, CString, CStr};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use gtk::prelude::*;
use glib::Propagation;

#[derive(Debug, Clone)]
pub struct PluginUIInfo {
    pub plugin_uri: String,
    pub ui_uri: String,
    pub ui_type: String,
    pub bundle_path: String,
    pub binary_path: String,
}

struct UIInstance {
    instance: *mut SuilInstance,
    window: gtk::Window,
}

unsafe impl Send for UIInstance {}

struct UridMapWrapper {
    urid_map: UridMapPtr,
    lv2_map: LV2_URID_Map,
}

unsafe impl Send for UridMapWrapper {}

pub struct PluginUIManager {
    host: *mut SuilHost,
    instances: Arc<Mutex<HashMap<String, UIInstance>>>,
    chain: Arc<Mutex<crate::audio::PluginChain>>,
    urid_map: UridMapPtr,
}

struct ControllerData {
    plugin_id: String,
    chain: Arc<Mutex<crate::audio::PluginChain>>,
}

extern "C" fn urid_map_callback(handle: *mut c_void, uri: *const i8) -> u32 {
    unsafe {
        let wrapper = &*(handle as *const UridMapWrapper);
        let cstr = CStr::from_ptr(uri);
        let mut map = wrapper.urid_map.lock().unwrap();
        map.map_uri(cstr)
    }
}

extern "C" fn urid_unmap_callback(handle: *mut c_void, urid: u32) -> *const i8 {
    unsafe {
        let wrapper = &*(handle as *const UridMapWrapper);
        let map = wrapper.urid_map.lock().unwrap();
        if let Some(uri) = map.unmap_uri(urid) {
            match CString::new(uri) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null(),
            }
        } else {
            std::ptr::null()
        }
    }
}

extern "C" fn port_write_callback(
    controller: SuilController,
    port_index: u32,
    buffer_size: u32,
    protocol: u32,
    buffer: *const c_void,
) {
    unsafe {
        let data = &*(controller as *const ControllerData);
        
        // Only handle float protocol (protocol == 0)
        if protocol == 0 && buffer_size == 4 {
            let value = *(buffer as *const f32);
            log::debug!("UI parameter change: plugin={}, port={}, value={}", 
                data.plugin_id, port_index, value);
            
            if let Ok(mut chain) = data.chain.lock() {
                chain.set_parameter_value(&data.plugin_id, port_index, value);
            }
        }
    }
}

extern "C" fn port_index_callback(
    _controller: SuilController,
    _port_symbol: *const std::ffi::c_char,
) -> u32 {
    0
}

impl PluginUIManager {
    pub fn new(chain: Arc<Mutex<crate::audio::PluginChain>>) -> Result<Self> {
        unsafe {
            suil::suil_init(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                SUIL_ARG_NONE,
            );
            
            let host = suil::suil_host_new(
                port_write_callback,
                Some(port_index_callback),
                None,
                None,
            );

            if host.is_null() {
                return Err(anyhow!("Failed to create Suil host"));
            }

            let urid_map = Arc::new(Mutex::new(UridMap::new()));

            Ok(Self {
                host,
                instances: Arc::new(Mutex::new(HashMap::new())),
                chain,
                urid_map,
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

        let ui_info = {
            let chain = self.chain.lock().map_err(|e| anyhow!("{}", e))?;
            chain.get_plugin_ui_info_by_id(plugin_id)?
        };

        log::info!(
            "Opening UI for plugin {}:\n  plugin_uri: {}\n  ui_uri: {}\n  ui_type: {}\n  bundle: {}\n  binary: {}",
            plugin_id,
            ui_info.plugin_uri,
            ui_info.ui_uri,
            ui_info.ui_type,
            ui_info.bundle_path,
            ui_info.binary_path
        );

        if ui_info.ui_type.contains("X11UI") {
            self.open_x11_ui(plugin_id, &ui_info)
        } else {
            Err(anyhow!("Unsupported UI type: {}", ui_info.ui_type))
        }
    }

    fn open_x11_ui(&mut self, plugin_id: &str, ui_info: &PluginUIInfo) -> Result<()> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(&format!("Plugin: {}", plugin_id));
        window.set_default_size(600, 400);
        
        let socket = gtk::Socket::new();
        window.add(&socket);
        
        window.show_all();
        
        while gtk::events_pending() {
            gtk::main_iteration();
        }
        
        let socket_xid = socket.id() as u64;
        log::info!("GtkSocket XID: {}", socket_xid);
        
        let controller_data = Box::new(ControllerData {
            plugin_id: plugin_id.to_string(),
            chain: self.chain.clone(),
        });
        let controller_ptr = Box::into_raw(controller_data) as SuilController;
        
        let instance_ptr: *mut SuilInstance;
        
        let (features, wrapper_ptr) = Self::build_features(socket_xid, self.urid_map.clone());
        let feature_ptrs = Self::features_to_ptrs(&features);
        
        unsafe {
            let container_type = CString::new("http://lv2plug.in/ns/extensions/ui#X11UI").unwrap();
            let plugin_uri = CString::new(ui_info.plugin_uri.as_str()).unwrap();
            let ui_uri = CString::new(ui_info.ui_uri.as_str()).unwrap();
            let ui_type = CString::new(ui_info.ui_type.as_str()).unwrap();
            let bundle_path = CString::new(ui_info.bundle_path.as_str()).unwrap();
            let binary_path = CString::new(ui_info.binary_path.as_str()).unwrap();

            log::debug!("Calling suil_instance_new...");
            instance_ptr = suil::suil_instance_new(
                self.host,
                controller_ptr,
                container_type.as_ptr(),
                plugin_uri.as_ptr(),
                ui_uri.as_ptr(),
                ui_type.as_ptr(),
                bundle_path.as_ptr(),
                binary_path.as_ptr(),
                feature_ptrs.as_ptr(),
            );
            log::debug!("suil_instance_new returned: {:?}", instance_ptr);

            if instance_ptr.is_null() {
                drop(Box::from_raw(controller_ptr as *mut ControllerData));
                drop(Box::from_raw(wrapper_ptr as *mut UridMapWrapper));
                return Err(anyhow!("Failed to instantiate UI"));
            }

            log::info!("UI instantiated successfully");

            let widget = suil::suil_instance_get_widget(instance_ptr);
            let widget_xid = widget as i64;
            log::info!("Plugin X11 window XID: {}", widget_xid);
            
            if widget_xid != 0 {
                socket.add_id(widget_xid as _);
                log::info!("Embedded X11 window into GtkSocket");
                
                let display = x11::xlib::XOpenDisplay(std::ptr::null());
                if !display.is_null() {
                    x11::xlib::XMapWindow(display, widget_xid as _);
                    x11::xlib::XFlush(display);
                    x11::xlib::XCloseDisplay(display);
                    log::debug!("Mapped plugin X11 window");
                }
            }
        }

        let instances_clone = self.instances.clone();
        let plugin_id_clone = plugin_id.to_string();
        let instance_ptr_for_close = instance_ptr;
        let controller_ptr_for_close = controller_ptr;
        let wrapper_ptr_for_close = wrapper_ptr;
        
        window.connect_delete_event(move |_w, _| {
            log::info!("UI window closed for {}", plugin_id_clone);
            if let Ok(mut insts) = instances_clone.lock() {
                unsafe {
                    suil::suil_instance_free(instance_ptr_for_close);
                    drop(Box::from_raw(controller_ptr_for_close as *mut ControllerData));
                    drop(Box::from_raw(wrapper_ptr_for_close as *mut UridMapWrapper));
                }
                insts.remove(&plugin_id_clone);
            }
            Propagation::Proceed
        });

        {
            let mut instances = self.instances.lock().unwrap();
            instances.insert(
                plugin_id.to_string(),
                UIInstance {
                    instance: instance_ptr,
                    window,
                },
            );
        }

        Ok(())
    }

    fn build_features(parent_xid: u64, urid_map: UridMapPtr) -> (Vec<Box<LV2_Feature>>, *mut UridMapWrapper) {
        let mut features: Vec<Box<LV2_Feature>> = Vec::new();
        
        let wrapper = Box::new(UridMapWrapper {
            urid_map: urid_map.clone(),
            lv2_map: LV2_URID_Map {
                handle: std::ptr::null_mut(),
                map: Some(urid_map_callback),
            },
        });
        
        let wrapper_ptr = Box::into_raw(wrapper);
        unsafe {
            (*wrapper_ptr).lv2_map.handle = wrapper_ptr as *mut c_void;
        }
        
        let urid_map_feature = Box::new(LV2_Feature {
            uri: LV2_URID__map.as_ptr() as *const i8,
            data: unsafe { &(*wrapper_ptr).lv2_map as *const _ as *mut c_void },
        });
        features.push(urid_map_feature);
        
        let parent_feature = Box::new(LV2_Feature {
            uri: LV2_UI__parent.as_ptr() as *const i8,
            data: parent_xid as *mut c_void,
        });
        features.push(parent_feature);
        
        let idle_feature = Box::new(LV2_Feature {
            uri: LV2_UI__idleInterface.as_ptr() as *const i8,
            data: std::ptr::null_mut(),
        });
        features.push(idle_feature);
        
        (features, wrapper_ptr)
    }

    fn features_to_ptrs(features: &[Box<LV2_Feature>]) -> Vec<*const LV2_Feature> {
        let mut ptrs: Vec<*const LV2_Feature> = features.iter().map(|f| f.as_ref() as *const _).collect();
        ptrs.push(std::ptr::null());
        ptrs
    }

    pub fn close_ui(&mut self, plugin_id: &str) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(instance) = instances.remove(plugin_id) {
            instance.window.close();
            unsafe {
                suil::suil_instance_free(instance.instance);
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
                suil::suil_instance_free(instance.instance);
            }
            log::info!("Closed UI for plugin {}", plugin_id);
        }
    }
    
    pub fn is_ui_open(&self, plugin_id: &str) -> bool {
        let instances = self.instances.lock().unwrap();
        instances.contains_key(plugin_id)
    }
}

impl Drop for PluginUIManager {
    fn drop(&mut self) {
        self.close_all();
        unsafe {
            suil::suil_host_free(self.host);
        }
    }
}

unsafe impl Send for PluginUIManager {}
