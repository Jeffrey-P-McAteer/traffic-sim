use anyhow::Result;
use log::info;
use std::time::Instant;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use traffic_sim::{
    config::SimulationConfig,
    simulation::{SimulationState, PerformanceTracker},
    graphics::GraphicsSystem,
    compute::{ComputeBackend, SimulationBackend},
};

struct Application {
    graphics: GraphicsSystem,
    simulation_state: SimulationState,
    compute_backend: ComputeBackend,
    performance_tracker: PerformanceTracker,
    paused: bool,
    last_frame_time: Instant,
    target_fps: f32,
    simulation_speed: f32,
}

impl Application {
    async fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        // Initialize logging
        env_logger::init();
        info!("Starting Traffic Simulator");
        
        // Load configuration
        let config = SimulationConfig::load_from_files("route.toml", "cars.toml")?;
        info!("Loaded configuration: {} cars max, route: {}", 
              config.cars.simulation.total_cars, 
              config.route.route.name);
        
        // Initialize graphics system
        let graphics = GraphicsSystem::new(event_loop).await?;
        info!("Graphics system initialized");
        
        // Initialize simulation state
        let dt = 1.0 / 60.0; // 60 FPS simulation timestep
        let simulation_state = SimulationState::new(dt);
        
        // Try to initialize GPU compute backend, fall back to CPU
        let compute_backend = match ComputeBackend::new_gpu(
            config.cars.clone(),
            config.route.clone(), 
            config.cars.random.seed
        ) {
            Ok(gpu_backend) => {
                info!("Using GPU compute backend: {}", gpu_backend.get_name());
                gpu_backend
            }
            Err(e) => {
                info!("GPU compute not available ({}), using CPU", e);
                ComputeBackend::new_cpu(
                    config.cars.clone(),
                    config.route.clone(),
                    config.cars.random.seed
                )
            }
        };
        
        info!("Compute backend: {}", compute_backend.get_name());
        
        // Initialize performance tracker
        let performance_tracker = PerformanceTracker::new(
            config.cars.performance.timing_samples as usize
        );
        
        Ok(Self {
            graphics,
            simulation_state,
            compute_backend,
            performance_tracker,
            paused: false,
            last_frame_time: Instant::now(),
            target_fps: 60.0,
            simulation_speed: 1.0,
        })
    }
    
    fn update(&mut self) -> Result<()> {
        if !self.paused {
            // Update simulation
            self.performance_tracker.start_simulation();
            
            // Scale timestep by simulation speed
            let original_dt = self.simulation_state.dt;
            self.simulation_state.dt = original_dt * self.simulation_speed;
            
            self.compute_backend.update(&mut self.simulation_state)?;
            
            // Restore original timestep
            self.simulation_state.dt = original_dt;
            
            self.performance_tracker.end_simulation();
        }
        
        Ok(())
    }
    
    fn render(&mut self) -> Result<()> {
        self.performance_tracker.start_render();
        
        // Create performance metrics
        let performance_metrics = crate::simulation::PerformanceMetrics {
            frame_time: self.performance_tracker.average_frame_time(),
            simulation_time: self.performance_tracker.average_simulation_time(),
            render_time: std::time::Duration::ZERO, // Will be updated by tracker
            cpu_utilization: 0.0,
            gpu_utilization: 0.0,
            memory_usage: 0,
        };
        
        self.graphics.render(&self.simulation_state, &performance_metrics)?;
        
        self.performance_tracker.end_render();
        
        Ok(())
    }
    
    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        // Handle graphics input first
        if self.graphics.handle_input(event) {
            return true;
        }
        
        // Handle application-specific input
        match event {
            WindowEvent::KeyboardInput { 
                event: winit::event::KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: winit::keyboard::PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } => {
                match keycode {
                    winit::keyboard::KeyCode::Space => {
                        self.paused = !self.paused;
                        info!("Simulation {}", if self.paused { "paused" } else { "resumed" });
                        true
                    }
                    winit::keyboard::KeyCode::KeyR => {
                        // Reset simulation
                        self.simulation_state = SimulationState::new(1.0 / 60.0);
                        info!("Simulation reset");
                        true
                    }
                    winit::keyboard::KeyCode::Digit1 => {
                        self.simulation_speed = 0.25;
                        info!("Simulation speed: 0.25x");
                        true
                    }
                    winit::keyboard::KeyCode::Digit2 => {
                        self.simulation_speed = 0.5;
                        info!("Simulation speed: 0.5x");
                        true
                    }
                    winit::keyboard::KeyCode::Digit3 => {
                        self.simulation_speed = 1.0;
                        info!("Simulation speed: 1.0x");
                        true
                    }
                    winit::keyboard::KeyCode::Digit4 => {
                        self.simulation_speed = 2.0;
                        info!("Simulation speed: 2.0x");
                        true
                    }
                    winit::keyboard::KeyCode::Digit5 => {
                        self.simulation_speed = 4.0;
                        info!("Simulation speed: 4.0x");
                        true
                    }
                    winit::keyboard::KeyCode::F1 => {
                        // Toggle performance display (would be implemented in UI)
                        info!("Performance display toggled");
                        true
                    }
                    _ => false
                }
            }
            _ => false
        }
    }
    
    fn update_frame_timing(&mut self) {
        let now = Instant::now();
        let _delta_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;
        
        // Limit frame rate if needed
        let target_frame_time = std::time::Duration::from_secs_f32(1.0 / self.target_fps);
        let elapsed = now.elapsed();
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
    }
}

fn main() -> Result<()> {
    // Create event loop
    let event_loop = EventLoop::new();
    
    // Create application
    let mut app = pollster::block_on(Application::new(&event_loop))?;
    
    info!("Starting main loop");
    
    // Main event loop
    event_loop.run(move |event, _, control_flow| {
        app.performance_tracker.start_frame();
        
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.graphics.window.id() => {
                if !app.handle_input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            info!("Close requested");
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::Resized(_physical_size) => {
                            // Handled in graphics system
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // Handled in graphics system
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == app.graphics.window.id() => {
                if let Err(e) = app.update() {
                    log::error!("Update error: {}", e);
                }
                
                if let Err(e) = app.render() {
                    log::error!("Render error: {}", e);
                }
            }
            Event::MainEventsCleared => {
                // Request redraw
                app.graphics.window.request_redraw();
                app.update_frame_timing();
            }
            _ => {}
        }
        
        app.performance_tracker.end_frame();
    });
}
