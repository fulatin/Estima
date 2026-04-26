use std::ffi::{c_char, c_void, CString};

pub type SuilController = *mut c_void;

pub type SuilWidget = *mut c_void;

pub type SuilPortWriteFunc = extern "C" fn(
    SuilController,
    u32,
    u32,
    u32,
    *const c_void,
);

pub type SuilPortIndexFunc = extern "C" fn(SuilController, *const c_char) -> u32;

pub type SuilPortSubscribeFunc = extern "C" fn(
    SuilController,
    u32,
    u32,
    *const crate::ffi::lv2_ui::LV2_Feature,
) -> u32;

pub type SuilPortUnsubscribeFunc = extern "C" fn(
    SuilController,
    u32,
    u32,
    *const crate::ffi::lv2_ui::LV2_Feature,
) -> u32;

#[repr(C)]
pub struct SuilHost {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SuilInstance {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn suil_init(
        argc: *mut i32,
        argv: *mut *mut *mut c_char,
        key: i32,
        ...
    );

    pub fn suil_ui_supported(
        host_type_uri: *const c_char,
        ui_type_uri: *const c_char,
    ) -> u32;

    pub fn suil_host_new(
        write_func: SuilPortWriteFunc,
        index_func: Option<SuilPortIndexFunc>,
        subscribe_func: Option<SuilPortSubscribeFunc>,
        unsubscribe_func: Option<SuilPortUnsubscribeFunc>,
    ) -> *mut SuilHost;

    pub fn suil_host_free(host: *mut SuilHost);

    pub fn suil_instance_new(
        host: *mut SuilHost,
        controller: SuilController,
        container_type_uri: *const c_char,
        plugin_uri: *const c_char,
        ui_uri: *const c_char,
        ui_type_uri: *const c_char,
        ui_bundle_path: *const c_char,
        ui_binary_path: *const c_char,
        features: *const *const crate::ffi::lv2_ui::LV2_Feature,
    ) -> *mut SuilInstance;

    pub fn suil_instance_free(instance: *mut SuilInstance);

    pub fn suil_instance_get_widget(instance: *mut SuilInstance) -> SuilWidget;

    pub fn suil_instance_port_event(
        instance: *mut SuilInstance,
        port_index: u32,
        buffer_size: u32,
        format: u32,
        buffer: *const c_void,
    );
}

pub const SUIL_ARG_NONE: i32 = 0;

pub fn uri_to_cstr(uri: &str) -> CString {
    CString::new(uri).expect("URI contains null byte")
}
