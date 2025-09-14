use crate::simulation::{SimulationState, PerformanceMetrics};
use crate::graphics::Viewport;
use anyhow::Result;

pub struct UiRenderer {
    // egui handles its own state, so we don't need much here
}

impl UiRenderer {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    pub fn render_egui(
        &mut self,
        ctx: &egui::Context,
        performance: &PerformanceMetrics,
        state: &SimulationState,
        viewport: &Viewport,
        paused: bool,
        simulation_speed: f32,
        frame_count: u64,
        route_file: &str,
        cars_file: &str,
        seed: Option<u64>,
        font_size: f32,
    ) {
        let fps = if !performance.frame_time.is_zero() {
            1.0 / performance.frame_time.as_secs_f32()
        } else {
            0.0
        };
        
        let status = if paused { "PAUSED" } else { "RUNNING" };
        
        // Configure font size for all text
        ctx.style_mut(|style| {
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(font_size, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(font_size, egui::FontFamily::Monospace),
            );
        });
        
        // Status overlay in the lower-left corner
        egui::Area::new(egui::Id::new("status_overlay"))
            .fixed_pos(egui::pos2(15.0, 15.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    // Semi-transparent background
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(
                        rect.expand(5.0),
                        5.0,
                        egui::Color32::from_black_alpha(160)
                    );
                    
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    
                    // Status section
                    ui.colored_label(
                        if paused { egui::Color32::YELLOW } else { egui::Color32::GREEN },
                        format!("Status: {}", status)
                    );
                    ui.label(format!("Cars: {}/{}", state.active_cars, state.total_spawned));
                    ui.label(format!("Time: {:.1}s", state.time));
                    ui.label(format!("Speed: {:.2}x", simulation_speed));
                    ui.label(format!("FPS: {:.0}", fps));
                    ui.label(format!("Frame: {}", frame_count));
                    
                    ui.add_space(10.0);
                    
                    // Files section
                    ui.label(format!("Route: {}", route_file));
                    ui.label(format!("Cars: {}", cars_file));
                    
                    // Seed information for reproducibility
                    match seed {
                        Some(s) => ui.label(format!("Seed: {}", s)),
                        None => ui.label("Seed: random"),
                    };
                    
                    ui.add_space(10.0);
                    
                    // Camera info
                    ui.label(format!("Zoom: {:.2}x", viewport.get_zoom()));
                    ui.label(format!("Pos: ({:.0}, {:.0})", 
                               viewport.get_position().x, viewport.get_position().y));
                });
            });
        
        // Controls help in the lower-left corner
        egui::Area::new(egui::Id::new("controls_overlay"))
            .fixed_pos(egui::pos2(15.0, 350.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    // Semi-transparent background
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(
                        rect.expand(5.0),
                        5.0,
                        egui::Color32::from_black_alpha(160)
                    );
                    
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    
                    ui.colored_label(egui::Color32::WHITE, "=== CONTROLS ===");
                    ui.label("Mouse: Drag=pan, Wheel=zoom");
                    ui.label("WASD/Arrows: Move camera");
                    ui.label("Home: Reset view");
                    ui.label("Space: Pause/Resume");
                    ui.label("1-9: Speed (1x-9x)");
                    ui.label("R: Reset simulation");
                    ui.label("ESC: Exit");
                    
                    ui.add_space(10.0);
                    
                    ui.colored_label(egui::Color32::WHITE, "=== SPAWN CARS ===");
                    ui.colored_label(egui::Color32::from_rgb(230, 50, 50), "A: Spawn Aggressive");
                    ui.colored_label(egui::Color32::from_rgb(50, 150, 230), "N: Spawn Normal");
                    ui.colored_label(egui::Color32::from_rgb(50, 200, 50), "C: Spawn Cautious");
                    ui.colored_label(egui::Color32::from_rgb(230, 125, 25), "E: Spawn Erratic");
                    ui.colored_label(egui::Color32::from_rgb(180, 50, 230), "S: Spawn Strategic");
                });
            });
        
        // Color legend in the lower-left corner
        egui::Area::new(egui::Id::new("legend_overlay"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(15.0, -15.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    // Semi-transparent background
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(
                        rect.expand(5.0),
                        5.0,
                        egui::Color32::from_black_alpha(160)
                    );
                    
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    
                    ui.colored_label(egui::Color32::WHITE, "=== CAR COLORS ===");
                    ui.colored_label(egui::Color32::from_rgb(230, 50, 50), "● Aggressive (Red)");
                    ui.colored_label(egui::Color32::from_rgb(50, 150, 230), "● Normal (Blue)");
                    ui.colored_label(egui::Color32::from_rgb(50, 200, 50), "● Cautious (Green)");
                    ui.colored_label(egui::Color32::from_rgb(230, 125, 25), "● Erratic (Orange)");
                    ui.colored_label(egui::Color32::from_rgb(180, 50, 230), "● Strategic (Purple)");
                    
                    ui.add_space(10.0);
                    
                    ui.colored_label(egui::Color32::WHITE, "=== HIGHWAY SYMBOLS ===");
                    ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "▲ Entry Points");
                    ui.colored_label(egui::Color32::from_rgb(200, 0, 0), "▲ Exit Points");
                    ui.colored_label(egui::Color32::from_rgb(230, 200, 50), "~ Merge Zones");
                    
                    ui.add_space(10.0);
                    
                    ui.colored_label(egui::Color32::WHITE, "=== LANES ===");
                    ui.colored_label(egui::Color32::WHITE, "Lane 1: Inner (Entry)");
                    ui.colored_label(egui::Color32::WHITE, "Lane 2: Middle (Travel)");
                    ui.colored_label(egui::Color32::WHITE, "Lane 3: Outer (Exit)");
                });
            });
    }
}

// Simple text overlay data structure for future GUI implementation
#[allow(dead_code)]
pub struct TextOverlay {
    pub text: String,
    pub position: (f32, f32),
    pub color: [f32; 3],
    pub size: f32,
}

impl TextOverlay {
    pub fn new(text: String, x: f32, y: f32) -> Self {
        Self {
            text,
            position: (x, y),
            color: [1.0, 1.0, 1.0], // White
            size: 16.0,
        }
    }
}

// Helper function to create performance overlay text
pub fn create_performance_overlay(
    performance: &PerformanceMetrics,
    state: &SimulationState,
    viewport: &Viewport,
) -> Vec<TextOverlay> {
    let fps = if !performance.frame_time.is_zero() {
        1.0 / performance.frame_time.as_secs_f32()
    } else {
        0.0
    };
    
    vec![
        TextOverlay::new(format!("FPS: {:.1}", fps), 10.0, 10.0),
        TextOverlay::new(
            format!("Frame: {:.1}ms", performance.frame_time.as_millis()),
            10.0,
            30.0,
        ),
        TextOverlay::new(
            format!("Sim: {:.1}ms", performance.simulation_time.as_millis()),
            10.0,
            50.0,
        ),
        TextOverlay::new(
            format!("Render: {:.1}ms", performance.render_time.as_millis()),
            10.0,
            70.0,
        ),
        TextOverlay::new(
            format!("Cars: {}/{}", state.active_cars, state.total_spawned),
            10.0,
            110.0,
        ),
        TextOverlay::new(
            format!("Time: {:.1}s", state.time),
            10.0,
            130.0,
        ),
        TextOverlay::new(
            format!("Zoom: {:.2}x", viewport.get_zoom()),
            10.0,
            170.0,
        ),
        TextOverlay::new(
            format!("Pos: ({:.0}, {:.0})", viewport.get_position().x, viewport.get_position().y),
            10.0,
            190.0,
        ),
    ]
}

// Control hints for the user
pub fn create_control_hints() -> Vec<TextOverlay> {
    vec![
        TextOverlay::new("=== VIEWPORT CONTROLS ===".to_string(), 10.0, 250.0),
        TextOverlay::new("Mouse Drag: Pan view".to_string(), 10.0, 270.0),
        TextOverlay::new("Mouse Wheel: Zoom in/out".to_string(), 10.0, 290.0),
        TextOverlay::new("WASD/Arrow Keys: Move camera".to_string(), 10.0, 310.0),
        TextOverlay::new("Home: Reset view to center".to_string(), 10.0, 330.0),
        TextOverlay::new("+/- Keys: Zoom in/out".to_string(), 10.0, 350.0),
        TextOverlay::new("=== SIMULATION CONTROLS ===".to_string(), 10.0, 380.0),
        TextOverlay::new("Space: Pause/Resume".to_string(), 10.0, 400.0),
        TextOverlay::new("R: Reset simulation".to_string(), 10.0, 420.0),
        TextOverlay::new("1-5: Set speed (0.25x - 4x)".to_string(), 10.0, 440.0),
        TextOverlay::new("ESC: Exit simulation".to_string(), 10.0, 460.0),
    ]
}