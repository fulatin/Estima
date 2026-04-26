use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use lv2_sys::{
    LV2_Feature, LV2UI_Controller, LV2UI_Descriptor, LV2UI_DescriptorFunction,
    LV2UI_Handle, LV2UI_Idle_Interface, LV2UI_Widget,
};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};
use std::sync::{Arc, Mutex};
use gtk::prelude::*;
use gtk::glib;
use glib::Propagation;

type LV2URID = u32;
type LV2URIDMapHandle = *mut c_void;

#[repr(C)]
struct LV2URIDMap {
    handle: LV2URIDMapHandle,
    map: Option<unsafe extern "C" fn(LV2URIDMapHandle, *const i8) -> LV2URID>,
}

#[repr(C)]
struct LV2OptionsOption {
    context: i32,
    subject: u32,
    key: LV2URID,
    size: u32,
    type_: LV2URID,
    value: *const c_void,
}

struct UridMap {
    map: HashMap<String, LV2URID>,
    next_id: LV2URID,
}

pub type ParameterChangeCallback = Arc<dyn Fn(&str, u32, f32) + Send + Sync>;

struct ControllerData {
    plugin_id: String,
    chain: Arc<Mutex<crate::audio::PluginChain>>,
    on_parameter_change: ParameterChangeCallback,
}

unsafe extern "C" fn urid_map_fn(handle: LV2URIDMapHandle, uri: *const i8) -> LV2URID {
    let urid_map = &mut *(handle as *mut UridMap);
    let uri_str = unsafe { CStr::from_ptr(uri) }.to_string_lossy().into_owned();
    
    if let Some(&id) = urid_map.map.get(&uri_str) {
        return id;
    }
    
    let id = urid_map.next_id;
    urid_map.next_id += 1;
    urid_map.map.insert(uri_str, id);
    id
}

extern "C" fn port_write_callback(
    controller: LV2UI_Controller,
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
            
            // Notify frontend
            (data.on_parameter_change)(&data.plugin_id, port_index, value);
        }
    }
}

pub struct LV2ExternalUIManager {
    instances: Arc<Mutex<HashMap<String, UIInstance>>>,
    chain: Arc<Mutex<crate::audio::PluginChain>>,
    controllers: Arc<Mutex<HashMap<String, Box<ControllerData>>>>,
    on_parameter_change: ParameterChangeCallback,
}

struct UIInstance {
    window: gtk::Window,
    library: Library,
    descriptor: LV2UI_Descriptor,
    ui_handle: LV2UI_Handle,
    idle_interface: Option<*const LV2UI_Idle_Interface>,
    timeout_id: Arc<Mutex<Option<glib::SourceId>>>,
    controller_data: *mut ControllerData,
}

unsafe impl Send for UIInstance {}

impl LV2ExternalUIManager {
    pub fn new(
        chain: Arc<Mutex<crate::audio::PluginChain>>,
        on_parameter_change: ParameterChangeCallback,
    ) -> Result<Self> {
        Ok(Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            chain,
            controllers: Arc::new(Mutex::new(HashMap::new())),
            on_parameter_change,
        })
    }

    pub fn open_ui(&mut self, plugin_id: &str) -> Result<()> {
        {
            let instances = self.instances.lock().unwrap();
            if instances.contains_key(plugin_id) {
                drop(instances);
                let instances = self.instances.lock().unwrap();
                if let Some(instance) = instances.get(plugin_id) {
                    instance.window.present();
                }
                return Ok(());
            }
        }

        let (plugin_uri, plugin_name, instance_handle, ui_info, current_values) = {
            let chain = self.chain.lock().map_err(|e| anyhow!("{}", e))?;
            let plugin = chain.get_plugin_by_id(plugin_id)
                .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
            
            let uri = plugin.info.uri.clone();
            let name = plugin.info.name.clone();
            
            // Check for known problematic plugins
            let known_issues = [
                "guitarix.sourceforge.net/plugins/gx_alembic",
                "guitarix.sourceforge.net/plugins/gx_studiopre",
                " guitarist.sourceforge.net/plugins/gx_sceleton",
            ];
            for issue in &known_issues {
                if uri.contains(issue) {
                    return Err(anyhow!("This plugin UI is not supported (known incompatibility with host embedding)"));
                }
            }
            
            let handle = plugin.instance.raw().instance().handle();
            
            let ui_info = chain.get_plugin_ui_info_by_id(plugin_id)?;
            
            // Get current parameter values
            let values: Vec<(u32, f32)> = plugin.parameters.iter()
                .zip(plugin.parameter_values.iter())
                .map(|(p, (_, v))| (p.port_index, *v))
                .collect();
            
            (uri, name, handle, ui_info, values)
        };

        log::info!(
            "Opening UI for plugin {} via direct loading: {}",
            plugin_id,
            plugin_uri
        );

        self.open_lv2_ui(plugin_id, &plugin_uri, &plugin_name, instance_handle, &ui_info, current_values)
    }

    fn open_lv2_ui(
        &mut self,
        plugin_id: &str,
        plugin_uri: &str,
        plugin_name: &str,
        instance_handle: lv2_sys::LV2_Handle,
        ui_info: &crate::audio::PluginUIInfo,
        current_values: Vec<(u32, f32)>,
    ) -> Result<()> {
        unsafe {
            let library = Library::new(&ui_info.binary_path)
                .map_err(|e| anyhow!("Failed to load UI library {}: {}", ui_info.binary_path, e))?;

            let descriptor_fn_opt: Symbol<LV2UI_DescriptorFunction> = library
                .get(b"lv2ui_descriptor\0")
                .map_err(|e| anyhow!("Failed to get lv2ui_descriptor: {}", e))?;

            let descriptor_fn = descriptor_fn_opt
                .ok_or_else(|| anyhow!("lv2ui_descriptor is null"))?;
            
            let descriptor_ptr = descriptor_fn(0);
            let descriptor = descriptor_ptr
                .as_ref()
                .ok_or_else(|| anyhow!("UI descriptor is null"))?
                .clone();

            let instantiate = descriptor.instantiate
                .ok_or_else(|| anyhow!("UI has no instantiate function"))?;

            let plugin_uri_c = CString::new(plugin_uri)?;
            let bundle_path_c = CString::new(&ui_info.bundle_path as &str)?;

            let window = gtk::Window::new(gtk::WindowType::Toplevel);
            window.set_title(&format!("Plugin: {}", plugin_name));
            window.set_default_size(400, 300);
            
            let socket_xid: u64;
            
            if ui_info.ui_type.contains("X11UI") {
                let socket = gtk::Socket::new();
                window.add(&socket);
                window.show_all();
                
                while gtk::events_pending() {
                    gtk::main_iteration();
                }
                
                socket_xid = socket.id() as u64;
                log::info!("GtkSocket XID: {}", socket_xid);
            } else {
                window.show_all();
                socket_xid = 0;
            }

            let urid_map: Box<UridMap> = Box::new(UridMap {
                map: HashMap::new(),
                next_id: 1000,
            });
            
            let urid_map_ptr = Box::into_raw(urid_map);
            
            let lv2_urid_map = Box::new(LV2URIDMap {
                handle: urid_map_ptr as *mut c_void,
                map: Some(urid_map_fn),
            });
            
            static URID_MAP_URI: &[u8] = b"http://lv2plug.in/ns/ext/urid#map\0";
            let urid_map_feature = LV2_Feature {
                URI: URID_MAP_URI.as_ptr() as *const i8,
                data: lv2_urid_map.as_ref() as *const _ as *mut c_void,
            };
            
            static INSTANCE_ACCESS_URI: &[u8] = b"http://lv2plug.in/ns/ext/instance-access\0";
            let instance_access_feature = LV2_Feature {
                URI: INSTANCE_ACCESS_URI.as_ptr() as *const i8,
                data: instance_handle as *mut c_void,
            };

            static OPTIONS_URI: &[u8] = b"http://lv2plug.in/ns/ext/options#options\0";
            let options_feature = LV2_Feature {
                URI: OPTIONS_URI.as_ptr() as *const i8,
                data: std::ptr::null_mut(),
            };

            static IDLE_INTERFACE_URI: &[u8] = b"http://lv2plug.in/ns/extensions/ui#idleInterface\0";
            let idle_feature = LV2_Feature {
                URI: IDLE_INTERFACE_URI.as_ptr() as *const i8,
                data: std::ptr::null_mut(),
            };

            static PARENT_URI: &[u8] = b"http://lv2plug.in/ns/extensions/ui#parent\0";
            let parent_feature = LV2_Feature {
                URI: PARENT_URI.as_ptr() as *const i8,
                data: socket_xid as *mut c_void,
            };
            
            let features: [*const LV2_Feature; 6] = [
                &instance_access_feature,
                &urid_map_feature,
                &options_feature,
                &idle_feature,
                &parent_feature,
                std::ptr::null(),
            ];

            let mut widget: LV2UI_Widget = std::ptr::null_mut();

            // Create controller data for callbacks
            let controller_data = Box::new(ControllerData {
                plugin_id: plugin_id.to_string(),
                chain: self.chain.clone(),
                on_parameter_change: self.on_parameter_change.clone(),
            });
            let controller_ptr = Box::into_raw(controller_data) as LV2UI_Controller;

            log::info!("Calling UI instantiate...");
            
            let ui_handle = instantiate(
                &descriptor,
                plugin_uri_c.as_ptr(),
                bundle_path_c.as_ptr(),
                Some(port_write_callback),
                controller_ptr,
                &mut widget as *mut _ as *mut *mut c_void,
                features.as_ptr(),
            );

            if ui_handle.is_null() {
                window.close();
                drop(Box::from_raw(controller_ptr as *mut ControllerData));
                drop(Box::from_raw(urid_map_ptr));
                return Err(anyhow!("Failed to instantiate UI"));
            }

            log::info!("UI instantiated, widget: {:?}", widget);

            // Send current parameter values to the UI
            if let Some(port_event) = descriptor.port_event {
                log::info!("Sending {} current parameter values to UI", current_values.len());
                for (port_index, value) in &current_values {
                    unsafe {
                        port_event(
                            ui_handle,
                            *port_index,
                            4,
                            0,  // float protocol
                            value as *const f32 as *const c_void,
                        );
                    }
                }
            }

            // Get idle interface for driving the UI
            let idle_interface = if let Some(ext_data) = descriptor.extension_data {
                static IDLE_URI: &[u8] = b"http://lv2plug.in/ns/extensions/ui#idleInterface\0";
                let idle_ptr = ext_data(IDLE_URI.as_ptr() as *const i8);
                if !idle_ptr.is_null() {
                    log::info!("Got idle interface");
                    Some(idle_ptr as *const LV2UI_Idle_Interface)
                } else {
                    log::info!("No idle interface available");
                    None
                }
            } else {
                None
            };

            // For X11UI, the widget is the X11 window ID
            if ui_info.ui_type.contains("X11UI") {
                let widget_xid = widget as i64;
                log::info!("Plugin widget XID: {}", widget_xid);
                
                if widget_xid != 0 && widget_xid != socket_xid as i64 {
                    let display = x11::xlib::XOpenDisplay(std::ptr::null());
                    if !display.is_null() {
                        x11::xlib::XMapWindow(display, widget_xid as _);
                        x11::xlib::XFlush(display);
                        x11::xlib::XCloseDisplay(display);
                        log::info!("Mapped X11 window");
                    }
                }
            }

            // Setup idle callback
            let timeout_id: Arc<Mutex<Option<glib::SourceId>>> = Arc::new(Mutex::new(None));
            
            if let Some(idle_iface_ptr) = idle_interface {
                let idle_iface = &*idle_iface_ptr;
                let ui_handle_for_idle = ui_handle;
                
                if let Some(idle_fn) = idle_iface.idle {
                    log::info!("Setting up idle callback");
                    
                    let idle_fn_for_closure = idle_fn;
                    let timeout_id_clone = timeout_id.clone();
                    
                    let source_id = glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                        let result = idle_fn_for_closure(ui_handle_for_idle);
                        if result != 0 {
                            log::info!("UI idle returned {}, stopping", result);
                            if let Ok(mut tid) = timeout_id_clone.lock() {
                                *tid = None;
                            }
                            glib::ControlFlow::Break
                        } else {
                            glib::ControlFlow::Continue
                        }
                    });
                    
                    if let Ok(mut tid) = timeout_id.lock() {
                        *tid = Some(source_id);
                    }
                }
            }

            let instances_clone = self.instances.clone();
            let plugin_id_clone = plugin_id.to_string();
            let timeout_id_for_close = timeout_id.clone();
            let descriptor_for_close = descriptor.clone();
            let ui_handle_for_close = ui_handle;
            let controller_ptr_for_close = controller_ptr;

            window.connect_delete_event(move |_w, _| {
                log::info!("UI window closed for {}", plugin_id_clone);
                
                if let Ok(mut tid) = timeout_id_for_close.lock() {
                    if let Some(source_id) = tid.take() {
                        source_id.remove();
                    }
                }
                
                if let Ok(mut insts) = instances_clone.lock() {
                    if let Some(inst) = insts.remove(&plugin_id_clone) {
                        unsafe {
                            if let Some(cleanup) = inst.descriptor.cleanup {
                                cleanup(inst.ui_handle);
                            }
                            drop(Box::from_raw(inst.controller_data));
                        }
                    }
                }
                Propagation::Proceed
            });

            let urid_map_box = Box::from_raw(urid_map_ptr);

            {
                let mut instances = self.instances.lock().unwrap();
                instances.insert(
                    plugin_id.to_string(),
                    UIInstance {
                        window,
                        library,
                        descriptor,
                        ui_handle,
                        idle_interface,
                        timeout_id,
                        controller_data: controller_ptr as *mut ControllerData,
                    },
                );
            }
            
            std::mem::forget(lv2_urid_map);

            Ok(())
        }
    }

    /// Notify UI about parameter change from backend
    pub fn notify_parameter_change(&self, plugin_id: &str, port_index: u32, value: f32) -> Result<()> {
        let instances = self.instances.lock().unwrap();
        
        if let Some(instance) = instances.get(plugin_id) {
            let port_event = instance.descriptor.port_event
                .ok_or_else(|| anyhow!("Plugin UI has no port_event callback"))?;
            
            unsafe {
                port_event(
                    instance.ui_handle,
                    port_index,
                    4,
                    0,  // float protocol
                    &value as *const f32 as *const c_void,
                );
            }
            
            log::debug!("Notified UI: plugin={}, port={}, value={}", plugin_id, port_index, value);
        }
        
        Ok(())
    }

    pub fn close_ui(&mut self, plugin_id: &str) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(instance) = instances.remove(plugin_id) {
            if let Ok(mut tid) = instance.timeout_id.lock() {
                if let Some(source_id) = tid.take() {
                    source_id.remove();
                }
            }
            
            instance.window.close();
            
            unsafe {
                if let Some(cleanup) = instance.descriptor.cleanup {
                    cleanup(instance.ui_handle);
                }
                drop(Box::from_raw(instance.controller_data));
            }
            
            log::info!("Closed UI for plugin {}", plugin_id);
        }
        Ok(())
    }

    pub fn close_all(&mut self) {
        let mut instances = self.instances.lock().unwrap();
        for (plugin_id, instance) in instances.drain() {
            if let Ok(mut tid) = instance.timeout_id.lock() {
                if let Some(source_id) = tid.take() {
                    source_id.remove();
                }
            }
            
            instance.window.close();
            
            unsafe {
                if let Some(cleanup) = instance.descriptor.cleanup {
                    cleanup(instance.ui_handle);
                }
                drop(Box::from_raw(instance.controller_data));
            }
            
            log::info!("Closed UI for plugin {}", plugin_id);
        }
    }

    pub fn is_ui_open(&self, plugin_id: &str) -> bool {
        let instances = self.instances.lock().unwrap();
        instances.contains_key(plugin_id)
    }
}

impl Drop for LV2ExternalUIManager {
    fn drop(&mut self) {
        self.close_all();
    }
}
