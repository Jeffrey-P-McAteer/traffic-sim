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
            .fixed_pos(egui::pos2(15.0, 280.0))
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
        
        // Get behavior counts for the legend
        let behavior_counts = state.get_behavior_counts();

        // Color legend in the lower-left corner (20% wider)
        egui::Area::new(egui::Id::new("legend_overlay"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(15.0, -15.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    // Set minimum width to be 20% wider than default
                    ui.set_min_width(240.0); // 20% wider than typical egui default (~200px)

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
                    ui.colored_label(egui::Color32::from_rgb(230, 50, 50),
                        format!("● Aggressive (Red): {}", behavior_counts.get("aggressive").unwrap_or(&0)));
                    ui.colored_label(egui::Color32::from_rgb(50, 150, 230),
                        format!("● Normal (Blue): {}", behavior_counts.get("normal").unwrap_or(&0)));
                    ui.colored_label(egui::Color32::from_rgb(50, 200, 50),
                        format!("● Cautious (Green): {}", behavior_counts.get("cautious").unwrap_or(&0)));
                    ui.colored_label(egui::Color32::from_rgb(230, 125, 25),
                        format!("● Erratic (Orange): {}", behavior_counts.get("erratic").unwrap_or(&0)));
                    ui.colored_label(egui::Color32::from_rgb(180, 50, 230),
                        format!("● Strategic (Purple): {}", behavior_counts.get("strategic").unwrap_or(&0)));
                    
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

        // Velocity distribution graph on the right side
        let velocity_distribution = state.get_velocity_distribution(16);
        let max_count = velocity_distribution.iter().cloned().max().unwrap_or(0) as f32;

        // Calculate max speed for bucket labels (convert m/s to mph: m/s * 2.237)
        let max_speed_ms = state.cars.iter()
            .map(|car| car.velocity.magnitude())
            .fold(0.0, f32::max);
        let max_speed_mph = max_speed_ms * 2.237;
        let bucket_size_mph = if max_speed_mph > 0.0 { max_speed_mph / 16.0 } else { 0.0 };

        egui::Area::new(egui::Id::new("velocity_graph"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-15.0, 15.0))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    // Semi-transparent background
                    let rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(392.0, 300.0) // Another 40% wider: 280 * 1.4 = 392
                    );
                    ui.painter().rect_filled(
                        rect.expand(5.0),
                        5.0,
                        egui::Color32::from_black_alpha(160)
                    );

                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

                    ui.colored_label(egui::Color32::WHITE, "=== VELOCITY DISTRIBUTION ===");
                    ui.add_space(5.0);

                    // Draw histogram
                    let graph_rect = egui::Rect::from_min_size(
                        ui.cursor().min + egui::vec2(10.0, 0.0),
                        egui::vec2(372.0, 200.0) // Another 40% wider: 260 * 1.4 + 8 = 372
                    );

                    // Draw background for graph
                    ui.painter().rect_filled(
                        graph_rect,
                        2.0,
                        egui::Color32::from_gray(30)
                    );

                    // Draw bars
                    let bar_width = graph_rect.width() / 16.0;
                    for (i, &count) in velocity_distribution.iter().enumerate() {
                        if count > 0 {
                            let bar_height = if max_count > 0.0 {
                                (count as f32 / max_count) * (graph_rect.height() - 20.0)
                            } else {
                                0.0
                            };

                            let bar_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    graph_rect.min.x + i as f32 * bar_width + 1.0,
                                    graph_rect.max.y - bar_height - 10.0
                                ),
                                egui::vec2(bar_width - 2.0, bar_height)
                            );

                            // Color bars based on speed range
                            let color = if i < 4 {
                                egui::Color32::from_rgb(100, 255, 100) // Slow = green
                            } else if i < 12 {
                                egui::Color32::from_rgb(255, 255, 100) // Medium = yellow
                            } else {
                                egui::Color32::from_rgb(255, 100, 100) // Fast = red
                            };

                            ui.painter().rect_filled(bar_rect, 1.0, color);

                            // Draw count label if there's room
                            if bar_height > 15.0 {
                                ui.painter().text(
                                    bar_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    count.to_string(),
                                    egui::FontId::new(10.0, egui::FontFamily::Monospace),
                                    egui::Color32::BLACK
                                );
                            }
                        }
                    }

                    // Draw speed labels underneath each bucket (staggered)
                    for i in 0..16 {
                        let bucket_center_x = graph_rect.min.x + (i as f32 + 0.5) * bar_width;
                        let speed_min_mph = i as f32 * bucket_size_mph;
                        let speed_max_mph = (i + 1) as f32 * bucket_size_mph;

                        // Draw middle value of the speed range
                        let label = if bucket_size_mph > 0.0 {
                            let middle_speed = (speed_min_mph + speed_max_mph) / 2.0;
                            format!("{:.0}", middle_speed)
                        } else {
                            "0".to_string()
                        };

                        // Stagger labels: even indices on first line, odd indices on second line
                        let y_offset = if i % 2 == 0 { 2.0 } else { 14.0 };

                        ui.painter().text(
                            egui::pos2(bucket_center_x, graph_rect.max.y + y_offset),
                            egui::Align2::CENTER_TOP,
                            label,
                            egui::FontId::new(9.0, egui::FontFamily::Monospace),
                            egui::Color32::WHITE
                        );
                    }

                    // Draw axes labels (positioned below staggered speed labels)
                    ui.painter().text(
                        egui::pos2(graph_rect.min.x, graph_rect.max.y + 28.0),
                        egui::Align2::LEFT_TOP,
                        "Speed (mph)",
                        egui::FontId::new(font_size * 0.8, egui::FontFamily::Monospace),
                        egui::Color32::WHITE
                    );

                    // Move cursor past the graph (extra space for speed labels)
                    ui.allocate_space(egui::vec2(392.0, 240.0));

                    ui.add_space(5.0);
                    ui.label(format!("Total cars: {}", state.active_cars));
                    ui.label(format!("Max speed: {:.1} mph", max_speed_mph));
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
