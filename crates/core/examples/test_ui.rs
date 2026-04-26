use estima_core::audio::PluginChain;

fn main() {
    let chain = PluginChain::new().unwrap();
    let plugins = chain.list_available_plugins();
    
    let ui_plugins: Vec<_> = plugins.iter().filter(|p| p.has_ui).collect();
    
    println!("Total plugins: {}", plugins.len());
    println!("Plugins with UI: {}", ui_plugins.len());
    println!();
    
    // Show specific plugins
    let search_terms = ["phaser", "delay", "reverb", "eq", "chorus"];
    for term in &search_terms {
        println!("=== Plugins matching '{}' with UI ===", term);
        for p in plugins.iter().filter(|p| p.name.to_lowercase().contains(term) && p.has_ui).take(5) {
            println!("  {} (has_ui: {})", p.name, p.has_ui);
        }
        println!();
    }
}
