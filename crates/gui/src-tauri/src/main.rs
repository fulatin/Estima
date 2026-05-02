use estima_gui_lib::run;

fn main() {
    // Fix Gdk-Message Error 71 on Wayland
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run();
}
