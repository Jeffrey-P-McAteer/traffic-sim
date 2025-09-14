use opencl3::{
    context::Context,
    device::{Device, get_all_devices, CL_DEVICE_TYPE_GPU},
    kernel::{ExecuteKernel, Kernel},
    memory::{Buffer, CL_MEM_READ_WRITE, CL_MEM_READ_ONLY},
    program::Program,
    command_queue::{CommandQueue, CL_QUEUE_PROFILING_ENABLE},
    types::CL_TRUE,
};

use crate::simulation::{SimulationState, TrafficManager, Car};
use crate::config::{CarsConfig, RouteConfig};
use anyhow::{Result, anyhow};
use super::SimulationBackend;
use std::ptr;

pub struct GpuBackend {
    context: Context,
    queue: CommandQueue,
    program: Program,
    physics_kernel: Kernel,
    traffic_manager: TrafficManager,
    car_buffer: Option<Buffer<u8>>,
    route_buffer: Buffer<u8>,
    max_cars: usize,
}

const PHYSICS_KERNEL_SOURCE: &str = r#"
// Car data structure (matches Rust Car struct layout)
typedef struct {
    float pos_x, pos_y;        // position
    float vel_x, vel_y;        // velocity  
    float acc_x, acc_y;        // acceleration
    float heading;             // heading angle
    float length, width;       // dimensions
    float max_accel, max_decel; // acceleration limits
    float preferred_speed;     // preferred speed
    uint current_lane;         // current lane
    uint target_lane;          // target lane (0 = no target)
    float lane_change_progress; // 0.0 to 1.0
    float following_distance_factor;
    float target_speed;
    float reaction_time;
    float last_lane_change_time;
    // Padding to align to 16-float boundary
    float padding[2];
} Car;

// Route parameters
typedef struct {
    float center_x, center_y;
    float inner_radius, outer_radius;
    float lane_width;
    uint lane_count;
    float speed_limit, min_speed;
    float following_distance, lane_change_time;
    float friction_coefficient;
    float emergency_brake_distance;
    float warning_distance;
    float safety_margin;
} RouteParams;

__kernel void update_physics(
    __global Car* cars,
    const __global RouteParams* route,
    const float dt,
    const uint car_count,
    const float simulation_time
) {
    const uint gid = get_global_id(0);
    if (gid >= car_count) return;
    
    __global Car* car = &cars[gid];
    const __global RouteParams* r = route;
    
    // Calculate current position on donut
    const float to_car_x = car->pos_x - r->center_x;
    const float to_car_y = car->pos_y - r->center_y;
    const float current_angle = atan2(to_car_y, to_car_x);
    const float current_radius = sqrt(to_car_x * to_car_x + to_car_y * to_car_y);
    
    // Calculate target lane radius
    const float lane_offset = ((float)car->current_lane - 1.0f) * r->lane_width;
    const float target_radius = r->inner_radius + r->lane_width * 0.5f + lane_offset;
    
    // Find nearest car in front for collision avoidance
    float min_front_distance = INFINITY;
    float front_car_speed = 0.0f;
    
    for (uint i = 0; i < car_count; i++) {
        if (i == gid) continue;
        
        const __global Car* other = &cars[i];
        // Only consider cars in same lane or target lane
        if (other->current_lane != car->current_lane && 
            (car->target_lane == 0 || other->current_lane != car->target_lane)) {
            continue;
        }
        
        const float other_to_car_x = other->pos_x - r->center_x;
        const float other_to_car_y = other->pos_y - r->center_y;
        const float other_angle = atan2(other_to_car_y, other_to_car_x);
        
        // Calculate angular distance (accounting for wrap-around)
        float angle_diff = other_angle - current_angle;
        if (angle_diff < 0.0f) angle_diff += 2.0f * M_PI_F;
        
        // Only consider cars in front (within PI radians)
        if (angle_diff > 0.0f && angle_diff < M_PI_F) {
            const float arc_distance = angle_diff * current_radius;
            if (arc_distance < min_front_distance) {
                min_front_distance = arc_distance;
                front_car_speed = sqrt(other->vel_x * other->vel_x + other->vel_y * other->vel_y);
            }
        }
    }
    
    // Calculate target speed based on traffic (matching CPU implementation)
    float target_speed = car->target_speed;
    
    // Use route collision avoidance parameters (from config)
    const float emergency_brake_distance = r->emergency_brake_distance;
    const float warning_distance = r->warning_distance;
    const float safety_margin = r->safety_margin;
    
    // Calculate following distance (matching CPU implementation)
    const float current_speed = sqrt(car->vel_x * car->vel_x + car->vel_y * car->vel_y);
    const float base_following_distance = r->following_distance * current_speed;
    const float following_distance = base_following_distance * car->following_distance_factor + safety_margin;
    
    // Apply collision avoidance logic
    if (min_front_distance != INFINITY) {
        if (min_front_distance < emergency_brake_distance) {
            target_speed = 0.0f; // Emergency brake
        } else if (min_front_distance < warning_distance) {
            const float brake_factor = (min_front_distance - emergency_brake_distance) / 
                                     (warning_distance - emergency_brake_distance);
            target_speed *= brake_factor;
        } else if (min_front_distance < following_distance) {
            // Maintain following distance - match front car speed
            target_speed = min(front_car_speed, target_speed);
        }
    }
    
    // Apply speed limits
    target_speed = clamp(target_speed, r->min_speed, r->speed_limit);
    
    // Calculate acceleration (reuse current_speed from above)
    const float speed_diff = target_speed - current_speed;
    const float accel_mag = (speed_diff > 0.0f) ? 
        min(speed_diff / dt, car->max_accel) : 
        max(speed_diff / dt, -car->max_decel);
    
    // Calculate tangential direction
    const float tangent_angle = current_angle + M_PI_F / 2.0f;
    const float tangent_x = -sin(tangent_angle);
    const float tangent_y = cos(tangent_angle);
    
    // Update velocity (tangential motion)  
    const float new_speed = max(0.0f, current_speed + accel_mag * dt);
    car->vel_x = tangent_x * new_speed;
    car->vel_y = tangent_y * new_speed;
    
    // Update position using angular motion (matching CPU implementation)
    const float angular_velocity = target_speed / target_radius;
    const float new_angle = current_angle + angular_velocity * dt;
    
    // Calculate new position on the circle
    car->pos_x = r->center_x + target_radius * cos(new_angle);
    car->pos_y = r->center_y + target_radius * sin(new_angle);
    
    // Update heading
    car->heading = atan2(car->vel_y, car->vel_x);
    
    // Update acceleration for recording
    car->acc_x = tangent_x * accel_mag;
    car->acc_y = tangent_y * accel_mag;
}
"#;

impl GpuBackend {
    pub fn new(
        cars_config: CarsConfig, 
        route_config: RouteConfig,
        seed: Option<u64>
    ) -> Result<Self> {
        // Get GPU device
        let device_ids = get_all_devices(CL_DEVICE_TYPE_GPU)
            .map_err(|e| anyhow!("Failed to get GPU devices: {}", e))?;
        
        if device_ids.is_empty() {
            return Err(anyhow!("No GPU devices found"));
        }
        
        let device = Device::new(device_ids[0]);
        let device_name = device.name().map_err(|e| anyhow!("Failed to get device name: {}", e))?;
        log::info!("Using GPU device: {}", device_name);
        
        // Create context and command queue
        let context = Context::from_device(&device)
            .map_err(|e| anyhow!("Failed to create OpenCL context: {}", e))?;
        
        let queue = CommandQueue::create_default(&context, CL_QUEUE_PROFILING_ENABLE)
            .map_err(|e| anyhow!("Failed to create command queue: {}", e))?;
        
        // Build program and kernel
        let program = Program::create_and_build_from_source(&context, PHYSICS_KERNEL_SOURCE, "")
            .map_err(|e| anyhow!("Failed to build OpenCL program: {}", e))?;
        
        let physics_kernel = Kernel::create(&program, "update_physics")
            .map_err(|e| anyhow!("Failed to create physics kernel: {}", e))?;
        
        // Create route parameters buffer
        let route_params = Self::create_route_params(&route_config, &cars_config.collision_avoidance);
        let mut route_buffer = unsafe {
            Buffer::create(&context, CL_MEM_READ_ONLY, std::mem::size_of::<RouteParams>(), ptr::null_mut())
                .map_err(|e| anyhow!("Failed to create route buffer: {}", e))?
        };
        
        // Upload route data
        unsafe {
            let route_bytes = std::slice::from_raw_parts(
                &route_params as *const _ as *const u8,
                std::mem::size_of::<RouteParams>()
            );
            queue.enqueue_write_buffer(&mut route_buffer, CL_TRUE, 0, route_bytes, &[])
        }
            .map_err(|e| anyhow!("Failed to write route data: {}", e))?;
        
        // Create traffic manager for CPU-side logic
        let traffic_manager = TrafficManager::new(cars_config.clone(), route_config, seed);
        
        let max_cars = cars_config.simulation.total_cars as usize;
        
        Ok(Self {
            context,
            queue,
            program,
            physics_kernel,
            traffic_manager,
            car_buffer: None,
            route_buffer,
            max_cars,
        })
    }
    
    fn create_route_params(route_config: &RouteConfig, collision_avoidance: &crate::config::CollisionAvoidance) -> RouteParams {
        let route = &route_config.route;
        let geom = &route.geometry;
        let rules = &route.traffic_rules;
        let surface = &route.surface;
        
        RouteParams {
            center_x: geom.center_x,
            center_y: geom.center_y,
            inner_radius: geom.inner_radius,
            outer_radius: geom.outer_radius,
            lane_width: geom.lane_width,
            lane_count: geom.lane_count,
            speed_limit: rules.speed_limit,
            min_speed: rules.min_speed,
            following_distance: rules.following_distance,
            lane_change_time: rules.lane_change_time,
            friction_coefficient: surface.friction_coefficient,
            emergency_brake_distance: collision_avoidance.emergency_brake_distance,
            warning_distance: collision_avoidance.warning_distance,
            safety_margin: collision_avoidance.safety_margin,
        }
    }
    
    fn upload_cars_to_gpu(&mut self, state: &SimulationState) -> Result<()> {
        if state.cars.is_empty() {
            return Ok(());
        }
        
        // Create or resize buffer if needed
        let buffer_size = self.max_cars * std::mem::size_of::<GpuCar>();
        if self.car_buffer.is_none() {
            self.car_buffer = Some(unsafe {
                Buffer::create(&self.context, CL_MEM_READ_WRITE, buffer_size, ptr::null_mut())
                    .map_err(|e| anyhow!("Failed to create car buffer: {}", e))?
            });
        }
        
        // Convert cars to GPU format
        let mut gpu_cars = vec![GpuCar::default(); self.max_cars];
        for (i, car) in state.cars.iter().enumerate() {
            if i < self.max_cars {
                gpu_cars[i] = GpuCar::from_car(car, state.time);
            }
        }
        
        // Upload to GPU
        if let Some(ref mut buffer) = self.car_buffer {
            unsafe {
                let car_bytes = std::slice::from_raw_parts(
                    gpu_cars.as_ptr() as *const u8,
                    gpu_cars.len() * std::mem::size_of::<GpuCar>()
                );
                self.queue.enqueue_write_buffer(buffer, CL_TRUE, 0, car_bytes, &[])
            }
                .map_err(|e| anyhow!("Failed to upload cars to GPU: {}", e))?;
        }
        
        Ok(())
    }
    
    fn download_cars_from_gpu(&mut self, state: &mut SimulationState) -> Result<()> {
        if let Some(ref buffer) = self.car_buffer {
            let mut gpu_cars = vec![GpuCar::default(); self.max_cars];
            
            unsafe {
                let car_bytes = std::slice::from_raw_parts_mut(
                    gpu_cars.as_mut_ptr() as *mut u8,
                    gpu_cars.len() * std::mem::size_of::<GpuCar>()
                );
                self.queue.enqueue_read_buffer(buffer, CL_TRUE, 0, car_bytes, &[])
            }
                .map_err(|e| anyhow!("Failed to download cars from GPU: {}", e))?;
            
            // Update car data
            for (i, car) in state.cars.iter_mut().enumerate() {
                if i < self.max_cars {
                    gpu_cars[i].update_car(car);
                }
            }
        }
        
        Ok(())
    }
}

impl SimulationBackend for GpuBackend {
    fn update(&mut self, state: &mut SimulationState) -> Result<()> {
        // Handle traffic management on CPU (spawning, despawning, behavior decisions)
        self.traffic_manager.update(state);
        
        if !state.cars.is_empty() {
            // Upload car data to GPU
            self.upload_cars_to_gpu(state)?;
            
            // Execute physics kernel
            if let Some(ref car_buffer) = self.car_buffer {
                let kernel_event = unsafe {
                    ExecuteKernel::new(&self.physics_kernel)
                        .set_arg(car_buffer)
                        .set_arg(&self.route_buffer)
                        .set_arg(&state.dt)
                        .set_arg(&(state.cars.len() as u32))
                        .set_arg(&state.time)
                        .set_global_work_size(state.cars.len())
                        .enqueue_nd_range(&self.queue)
                        .map_err(|e| anyhow!("Failed to execute physics kernel: {}", e))?
                };
                
                // Wait for completion
                kernel_event.wait()
                    .map_err(|e| anyhow!("Failed to wait for kernel completion: {}", e))?;
            }
            
            // Download updated car data
            self.download_cars_from_gpu(state)?;
        }
        
        Ok(())
    }
    
    fn get_name(&self) -> &'static str {
        "OpenCL GPU"
    }
    
    fn supports_gpu(&self) -> bool {
        true
    }
}

impl GpuBackend {
    pub fn spawn_manual_car(&mut self, behavior_name: &str, state: &mut SimulationState) {
        self.traffic_manager.spawn_manual_car(behavior_name, state);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RouteParams {
    center_x: f32,
    center_y: f32,
    inner_radius: f32,
    outer_radius: f32,
    lane_width: f32,
    lane_count: u32,
    speed_limit: f32,
    min_speed: f32,
    following_distance: f32,
    lane_change_time: f32,
    friction_coefficient: f32,
    emergency_brake_distance: f32,
    warning_distance: f32,
    safety_margin: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct GpuCar {
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    acc_x: f32,
    acc_y: f32,
    heading: f32,
    length: f32,
    width: f32,
    max_accel: f32,
    max_decel: f32,
    preferred_speed: f32,
    current_lane: u32,
    target_lane: u32,
    lane_change_progress: f32,
    following_distance_factor: f32,
    target_speed: f32,
    reaction_time: f32,
    last_lane_change_time: f32,
    padding: [f32; 2],
}

impl GpuCar {
    fn from_car(car: &Car, _simulation_time: f32) -> Self {
        Self {
            pos_x: car.position.x,
            pos_y: car.position.y,
            vel_x: car.velocity.x,
            vel_y: car.velocity.y,
            acc_x: car.acceleration.x,
            acc_y: car.acceleration.y,
            heading: car.heading,
            length: car.length,
            width: car.width,
            max_accel: car.max_acceleration,
            max_decel: car.max_deceleration,
            preferred_speed: car.preferred_speed,
            current_lane: car.current_lane,
            target_lane: car.target_lane.unwrap_or(0),
            lane_change_progress: car.lane_change_progress,
            following_distance_factor: car.behavior.following_distance_factor,
            target_speed: car.behavior.target_speed,
            reaction_time: car.behavior.reaction_time,
            last_lane_change_time: car.behavior.last_lane_change_time,
            padding: [0.0; 2],
        }
    }
    
    fn update_car(&self, car: &mut Car) {
        car.position.x = self.pos_x;
        car.position.y = self.pos_y;
        car.velocity.x = self.vel_x;
        car.velocity.y = self.vel_y;
        car.acceleration.x = self.acc_x;
        car.acceleration.y = self.acc_y;
        car.heading = self.heading;
        car.lane_change_progress = self.lane_change_progress;
        
        // Update target lane if changed
        if self.target_lane != 0 {
            car.target_lane = Some(self.target_lane);
        } else {
            car.target_lane = None;
        }
        
        // Update behavior state
        car.behavior.target_speed = self.target_speed;
        car.behavior.last_lane_change_time = self.last_lane_change_time;
    }
}

