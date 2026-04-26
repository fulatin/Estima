use std::ffi::c_void;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

pub type LV2UI_Handle = *mut c_void;
pub type LV2UI_Widget = *mut c_void;
pub type LV2UI_Idle_Handle = *mut c_void;

#[repr(C)]
pub struct LV2_Feature {
    pub uri: *const i8,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct LV2UI_Idle_Interface {
    pub idle: Option<unsafe extern "C" fn(LV2UI_Handle)>,
}

pub type LV2_URID = u32;
pub type LV2_URID_Map_Handle = *mut c_void;
pub type LV2_URID_Unmap_Handle = *mut c_void;

#[repr(C)]
pub struct LV2_URID_Map {
    pub handle: LV2_URID_Map_Handle,
    pub map: Option<unsafe extern "C" fn(LV2_URID_Map_Handle, *const i8) -> LV2_URID>,
}

#[repr(C)]
pub struct LV2_URID_Unmap {
    pub handle: LV2_URID_Unmap_Handle,
    pub unmap: Option<unsafe extern "C" fn(LV2_URID_Unmap_Handle, LV2_URID) -> *const i8>,
}

pub struct UridMap {
    pub map: HashMap<String, LV2_URID>,
    pub reverse: HashMap<LV2_URID, String>,
    pub next_id: LV2_URID,
}

impl UridMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            reverse: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn map_uri(&mut self, uri: &CStr) -> LV2_URID {
        let uri_str = uri.to_string_lossy().into_owned();
        if let Some(&id) = self.map.get(&uri_str) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.map.insert(uri_str.clone(), id);
        self.reverse.insert(id, uri_str);
        id
    }
    
    pub fn unmap_uri(&self, id: LV2_URID) -> Option<&str> {
        self.reverse.get(&id).map(|s| s.as_str())
    }
}

pub type UridMapPtr = Arc<Mutex<UridMap>>;

pub const LV2_URID__map: &[u8] = b"http://lv2plug.in/ns/ext/urid#map\0";
pub const LV2_URID__unmap: &[u8] = b"http://lv2plug.in/ns/ext/urid#unmap\0";

pub const LV2_UI__parent: &[u8] = b"http://lv2plug.in/ns/extensions/ui#parent\0";
pub const LV2_UI__idleInterface: &[u8] = b"http://lv2plug.in/ns/extensions/ui#idleInterface\0";
pub const LV2_UI__instanceAccess: &[u8] = b"http://lv2plug.in/ns/ext#instance-access\0";
pub const LV2_UI__floatProtocol: &[u8] = b"http://lv2plug.in/ns/extensions/ui#floatProtocol\0";
pub const LV2_UI__Gtk3UI: &[u8] = b"http://lv2plug.in/ns/extensions/ui#Gtk3UI\0";
pub const LV2_UI__X11UI: &[u8] = b"http://lv2plug.in/ns/extensions/ui#X11UI\0";

pub const LV2_UI_URI: &str = "http://lv2plug.in/ns/extensions/ui#";
