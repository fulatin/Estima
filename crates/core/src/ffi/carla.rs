use std::ffi::{c_char, c_int, c_void};

pub type CarlaEngineHandle = *mut c_void;
pub type CarlaPluginId = u32;

#[repr(C)]
pub enum CarlaBackendEngineMode {
    EngineModeSingleClient = 0,
    EngineModeMultiClient = 1,
    EngineModeBridge = 2,
}

#[repr(C)]
pub enum CarlaPluginType {
    PluginTypeNone = 0,
    PluginTypeInternal = 1,
    PluginTypeLV2 = 2,
    PluginTypeAU = 3,
    PluginTypeVST2 = 4,
    PluginTypeVST3 = 5,
    PluginTypeCLAP = 6,
    PluginTypeJSFX = 7,
    PluginTypeSFZ = 8,
    PluginTypeSF2 = 9,
}

#[repr(C)]
pub struct CarlaPluginInfo {
    pub type_: CarlaPluginType,
    pub name: *const c_char,
    pub label: *const c_char,
    pub maker: *const c_char,
    pub copyright: *const c_char,
    pub unique_id: i64,
    pub hints: u64,
}

unsafe extern "C" {
    pub fn carla_engine_init(
        driver_name: *const c_char,
        client_name: *const c_char,
    ) -> CarlaEngineHandle;
    
    pub fn carla_engine_close(handle: CarlaEngineHandle) -> bool;
    
    pub fn carla_engine_idle(handle: CarlaEngineHandle);
    
    pub fn carla_add_plugin(
        handle: CarlaEngineHandle,
        btype: CarlaBackendEngineMode,
        ptype: CarlaPluginType,
        filename: *const c_char,
        name: *const c_char,
        label: *const c_char,
        unique_id: i64,
        extra_stuff: *const c_void,
        options: u64,
    ) -> c_int;
    
    pub fn carla_remove_plugin(handle: CarlaEngineHandle, plugin_id: CarlaPluginId) -> bool;
    
    pub fn carla_show_custom_ui(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        show: bool,
    ) -> bool;
    
    pub fn carla_embed_custom_ui(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        win_id: u64,
    ) -> bool;
    
    pub fn carla_x11_reparent_window(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        display: *mut c_void,
        win_id: u64,
    );
    
    pub fn carla_set_custom_ui_title(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        title: *const c_char,
    );
    
    pub fn carla_get_current_plugin_count(handle: CarlaEngineHandle) -> u32;
    
    pub fn carla_get_plugin_info(handle: CarlaEngineHandle, plugin_id: CarlaPluginId) -> *const CarlaPluginInfo;
    
    pub fn carla_set_parameter_value(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        parameter_id: u32,
        value: f32,
    ) -> bool;
    
    pub fn carla_get_current_parameter_value(
        handle: CarlaEngineHandle,
        plugin_id: CarlaPluginId,
        parameter_id: u32,
    ) -> f32;
}
