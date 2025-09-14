use nalgebra::{Vector2, Point2};
use std::time::{Duration, Instant};

pub mod physics;
pub mod behavior;
pub mod traffic;

pub use physics::*;
pub use behavior::*;
pub use traffic::*;

pub type Vec2 = Vector2<f32>;
pub type Point = Point2<f32>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarId(pub usize);

#[derive(Debug, Clone)]
pub struct Car {
    pub id: CarId,
    pub position: Point,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub heading: f32,
    pub length: f32,
    pub width: f32,
    pub max_acceleration: f32,
    pub max_deceleration: f32,
    pub preferred_speed: f32,
    pub current_lane: u32,
    pub target_lane: Option<u32>,
    pub lane_change_progress: f32,
    pub behavior: BehaviorState,
    pub car_type: String,
}

#[derive(Debug, Clone)]
pub struct BehaviorState {
    pub following_distance_factor: f32,
    pub lane_change_frequency: f32,
    pub speed_variance: f32,
    pub reaction_time: f32,
    pub exit_probability: f32,
    pub last_lane_change_time: f32,
    pub target_speed: f32,
}

#[derive(Debug, Clone)]
pub struct SimulationState {
    pub cars: Vec<Car>,
    pub time: f32,
    pub dt: f32,
    pub total_spawned: u32,
    pub active_cars: u32,
}

impl SimulationState {
    pub fn new(dt: f32) -> Self {
        Self {
            cars: Vec::new(),
            time: 0.0,
            dt,
            total_spawned: 0,
            active_cars: 0,
        }
    }
    
    pub fn add_car(&mut self, car: Car) {
        self.cars.push(car);
        self.total_spawned += 1;
        self.active_cars += 1;
    }
    
    pub fn remove_car(&mut self, id: CarId) {
        if let Some(pos) = self.cars.iter().position(|c| c.id == id) {
            self.cars.remove(pos);
            self.active_cars = self.active_cars.saturating_sub(1);
        }
    }
    
    pub fn get_car(&self, id: CarId) -> Option<&Car> {
        self.cars.iter().find(|c| c.id == id)
    }
    
    pub fn get_car_mut(&mut self, id: CarId) -> Option<&mut Car> {
        self.cars.iter_mut().find(|c| c.id == id)
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub frame_time: Duration,
    pub simulation_time: Duration,
    pub render_time: Duration,
    pub cpu_utilization: f32,
    pub gpu_utilization: f32,
    pub memory_usage: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            frame_time: Duration::ZERO,
            simulation_time: Duration::ZERO,
            render_time: Duration::ZERO,
            cpu_utilization: 0.0,
            gpu_utilization: 0.0,
            memory_usage: 0,
        }
    }
}

#[derive(Debug)]
pub struct PerformanceTracker {
    samples: Vec<PerformanceMetrics>,
    max_samples: usize,
    current_frame_start: Option<Instant>,
    current_sim_start: Option<Instant>,
    current_render_start: Option<Instant>,
}

impl PerformanceTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            current_frame_start: None,
            current_sim_start: None,
            current_render_start: None,
        }
    }
    
    pub fn start_frame(&mut self) {
        self.current_frame_start = Some(Instant::now());
    }
    
    pub fn start_simulation(&mut self) {
        self.current_sim_start = Some(Instant::now());
    }
    
    pub fn end_simulation(&mut self) {
        if let Some(start) = self.current_sim_start.take() {
            let duration = start.elapsed();
            if let Some(current) = self.samples.last_mut() {
                current.simulation_time = duration;
            }
        }
    }
    
    pub fn start_render(&mut self) {
        self.current_render_start = Some(Instant::now());
    }
    
    pub fn end_render(&mut self) {
        if let Some(start) = self.current_render_start.take() {
            let duration = start.elapsed();
            if let Some(current) = self.samples.last_mut() {
                current.render_time = duration;
            }
        }
    }
    
    pub fn end_frame(&mut self) {
        if let Some(start) = self.current_frame_start.take() {
            let frame_time = start.elapsed();
            
            let metrics = PerformanceMetrics {
                frame_time,
                simulation_time: self.samples.last()
                    .map(|s| s.simulation_time)
                    .unwrap_or(Duration::ZERO),
                render_time: self.samples.last()
                    .map(|s| s.render_time)
                    .unwrap_or(Duration::ZERO),
                cpu_utilization: 0.0, // TODO: Implement CPU monitoring
                gpu_utilization: 0.0, // TODO: Implement GPU monitoring
                memory_usage: 0,      // TODO: Implement memory monitoring
            };
            
            if self.samples.len() >= self.max_samples {
                self.samples.remove(0);
            }
            self.samples.push(metrics);
        }
    }
    
    pub fn average_frame_time(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.samples.iter().map(|s| s.frame_time).sum();
        total / self.samples.len() as u32
    }
    
    pub fn average_simulation_time(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.samples.iter().map(|s| s.simulation_time).sum();
        total / self.samples.len() as u32
    }
    
    pub fn fps(&self) -> f32 {
        let avg_frame_time = self.average_frame_time();
        if avg_frame_time.is_zero() {
            return 0.0;
        }
        1.0 / avg_frame_time.as_secs_f32()
    }
}