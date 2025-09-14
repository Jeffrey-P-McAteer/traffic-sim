use anyhow::Result;
use log::info;
use std::time::Instant;
use clap::{Parser, ValueEnum};
use winit::{
    event::*,
    event_loop::EventLoop,
};

use traffic_sim::{
    config::SimulationConfig,
    simulation::{SimulationState, PerformanceTracker},
    graphics::GraphicsSystem,
    compute::{ComputeBackend, SimulationBackend},
};

#[derive(Parser)]
#[command(name = "traffic-sim")]
#[command(about = "GPU-accelerated traffic simulation with interactive visualization")]
struct Args {
    /// Simulation compute backend
    #[arg(short, long, value_enum, default_value_t = Backend::Cpu)]
    backend: Backend,
    
    /// Route configuration file
    #[arg(short, long, default_value = "route.toml")]
    route: String,
    
    /// Cars configuration file
    #[arg(short, long, default_value = "cars.toml")]
    cars: String,
    
    /// Random seed for reproducible simulations
    #[arg(short, long)]
    seed: Option<u64>,
    
    /// Enable verbose logging for detailed simulation progress
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Backend {
    /// CPU-based simulation
    Cpu,
    /// OpenCL GPU-accelerated simulation
    Gpu,
}

struct Application {
    graphics: GraphicsSystem,
    simulation_state: SimulationState,
    compute_backend: ComputeBackend,
    performance_tracker: PerformanceTracker,
    paused: bool,
    last_frame_time: Instant,
    target_fps: f32,
    simulation_speed: f32,
    verbose: bool,
    route_file: String,
    cars_file: String,
    frame_count: u64,
    should_exit: bool,
}

impl Application {
    async fn new(args: &Args, event_loop: Option<&EventLoop<()>>) -> Result<Self> {
        // Initialize logging
        env_logger::Builder::from_default_env()
            .filter_level(if args.verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info })
            .init();
        info!("Starting Traffic Simulator");
        
        // Load configuration
        if args.verbose {
            info!("Loading route configuration from: {}", &args.route);
        }
        let config = SimulationConfig::load_from_files(&args.route, &args.cars)?;
        info!("Loaded configuration: {} cars max, route: {}", 
              config.cars.simulation.total_cars, 
              config.route.route.name);
        
        if args.verbose {
            info!("Route details: {} lanes, {:.1}m inner radius, {:.1}m outer radius", 
                  config.route.route.geometry.lane_count,
                  config.route.route.geometry.inner_radius,
                  config.route.route.geometry.outer_radius);
            info!("Traffic rules: {:.1} km/h speed limit, {:.1}m following distance", 
                  config.route.route.traffic_rules.speed_limit * 3.6,
                  config.route.route.traffic_rules.following_distance);
            info!("Car types loaded: {}", config.cars.car_types.len());
            info!("Behavior patterns loaded: {}", config.cars.behavior.len());
            info!("Spawn configuration: {} total cars, {:.1} cars/s spawn rate", 
                  config.cars.simulation.total_cars,
                  config.cars.simulation.spawn_rate);
        }
        
        // Initialize graphics system
        let graphics = match event_loop {
            Some(event_loop) => {
                let graphics = GraphicsSystem::new(event_loop).await?;
                info!("Graphics system initialized");
                graphics
            }
            None => return Err(anyhow::anyhow!("Event loop required for GUI application")),
        };
        
        // Initialize simulation state
        let dt = 1.0 / 60.0; // 60 FPS simulation timestep
        let simulation_state = SimulationState::new(dt);
        
        // Use seed from args or config
        let seed = args.seed.or(config.cars.random.seed);
        
        // Initialize compute backend based on CLI argument
        let compute_backend = match args.backend {
            Backend::Cpu => {
                let backend = ComputeBackend::new_cpu(
                    config.cars.clone(),
                    config.route.clone(),
                    seed
                );
                info!("✓ CPU Backend: {}", backend.get_name());
                backend
            }
            Backend::Gpu => {
                match ComputeBackend::new_gpu(
                    config.cars.clone(),
                    config.route.clone(),
                    seed
                ) {
                    Ok(backend) => {
                        info!("✓ GPU Backend: {} (OpenCL detected and initialized)", backend.get_name());
                        backend
                    }
                    Err(e) => {
                        info!("✗ GPU Backend: OpenCL not available ({e})");
                        info!("↳ Falling back to CPU backend");
                        let backend = ComputeBackend::new_cpu(
                            config.cars.clone(),
                            config.route.clone(),
                            seed
                        );
                        info!("✓ CPU Backend: {}", backend.get_name());
                        backend
                    }
                }
            }
        };
        
        // Initialize performance tracker
        let performance_tracker = PerformanceTracker::new(
            config.cars.performance.timing_samples as usize
        );
        
        // Display startup information
        info!("=== Simulation Configuration ===");
        info!("Graphics: GPU accelerated (wgpu)");
        info!("Compute: {}", compute_backend.get_name());
        info!("Route: {} ({})", config.route.route.name, config.route.route.description);
        info!("Max Cars: {}", config.cars.simulation.total_cars);
        if let Some(seed) = seed {
            info!("Random Seed: {}", seed);
        }
        
        if args.verbose {
            info!("Verbose logging enabled - detailed simulation progress will be shown");
            info!("Physics timestep: {:.3}s ({:.1} Hz)", dt, 1.0 / dt);
            info!("Performance tracking: {} samples", config.cars.performance.timing_samples);
        }
        
        Ok(Self {
            graphics,
            simulation_state,
            compute_backend,
            performance_tracker,
            paused: false,
            last_frame_time: Instant::now(),
            target_fps: 60.0,
            simulation_speed: 1.0,
            verbose: args.verbose,
            route_file: args.route.clone(),
            cars_file: args.cars.clone(),
            frame_count: 0,
            should_exit: false,
        })
    }
    
    fn update(&mut self) -> Result<()> {
        if !self.paused {
            // Update simulation
            self.performance_tracker.start_simulation();
            
            // Scale timestep by simulation speed
            let original_dt = self.simulation_state.dt;
            self.simulation_state.dt = original_dt * self.simulation_speed;
            
            // Verbose logging for simulation state changes
            let prev_car_count = self.simulation_state.active_cars as usize;
            
            self.compute_backend.update(&mut self.simulation_state)?;
            
            // Update active car count and log changes
            self.simulation_state.active_cars = self.simulation_state.cars.len() as u32;
            
            if self.verbose && self.simulation_state.cars.len() != prev_car_count {
                if self.simulation_state.cars.len() > prev_car_count {
                    log::debug!("Car spawned: total cars = {}", self.simulation_state.cars.len());
                } else if self.simulation_state.cars.len() < prev_car_count {
                    log::debug!("Car despawned: total cars = {}", self.simulation_state.cars.len());
                }
            }
            
            // Restore original timestep
            self.simulation_state.dt = original_dt;
            
            self.performance_tracker.end_simulation();
        }
        
        // Increment frame counter
        self.frame_count += 1;
        
        Ok(())
    }
    
    fn render(&mut self) -> Result<()> {
        self.performance_tracker.start_render();
        
        // Create performance metrics
        let performance_metrics = traffic_sim::simulation::PerformanceMetrics {
            frame_time: self.performance_tracker.average_frame_time(),
            simulation_time: self.performance_tracker.average_simulation_time(),
            render_time: std::time::Duration::ZERO, // Will be updated by tracker
            cpu_utilization: 0.0,
            gpu_utilization: 0.0,
            memory_usage: 0,
        };
        
        self.graphics.render(
            &self.simulation_state, 
            &performance_metrics,
            self.paused,
            self.simulation_speed,
            self.frame_count,
            &self.route_file,
            &self.cars_file
        )?;
        
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
                    winit::keyboard::KeyCode::Escape => {
                        info!("ESC pressed - exiting simulation");
                        self.should_exit = true;
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

async fn run_simulation(args: Args) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = Application::new(&args, Some(&event_loop)).await?;
    
    info!("Starting interactive mode...");
    
    event_loop.run(move |event, control_flow| {
        app.performance_tracker.start_frame();
        
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                if window_id == app.graphics.window.id() {
                    if !app.handle_input(event) {
                        match event {
                            WindowEvent::CloseRequested => {
                                info!("Close requested");
                                control_flow.exit();
                            }
                            WindowEvent::RedrawRequested => {
                                if let Err(e) = app.update() {
                                    log::error!("Update error: {}", e);
                                }
                                
                                if let Err(e) = app.render() {
                                    log::error!("Render error: {}", e);
                                }
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
                
                // Check for exit flag
                if app.should_exit {
                    control_flow.exit();
                }
            }
            Event::AboutToWait => {
                // Request redraw
                app.graphics.window.request_redraw();
                app.update_frame_timing();
            }
            _ => {}
        }
        
        app.performance_tracker.end_frame();
    })?;
    Ok(())
}


fn main() -> Result<()> {
    let args = Args::parse();
    
    pollster::block_on(async {
        run_simulation(args).await
    })
}
