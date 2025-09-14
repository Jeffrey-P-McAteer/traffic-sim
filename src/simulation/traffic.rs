use super::{Car, CarId, SimulationState, BehaviorEngine};
use crate::config::{CarsConfig, RouteConfig, CarType};
use nalgebra::{Point2, Vector2};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;

pub struct TrafficManager {
    car_types: Vec<CarType>,
    route: RouteConfig,
    cars_config: CarsConfig,
    behavior_engine: BehaviorEngine,
    next_car_id: usize,
    spawn_timers: HashMap<String, f32>, // Entry ID -> time until next spawn
    rng: StdRng,
}

impl TrafficManager {
    pub fn new(cars_config: CarsConfig, route: RouteConfig, seed: Option<u64>) -> Self {
        let behavior_engine = BehaviorEngine::new(&cars_config, route.clone(), seed);
        
        let rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_entropy()
        };
        
        // Initialize spawn timers based on spawn_rate
        let mut spawn_timers = HashMap::new();
        let base_interval = 1.0 / cars_config.simulation.spawn_rate; // Convert rate to interval
        
        for entry in &route.route.entries {
            // Use entry-specific intervals if configured, otherwise use spawn rate
            let interval = cars_config.traffic_flow.entry_intervals
                .iter()
                .find(|ei| ei.entry_id == entry.id)
                .map(|ei| rng.clone().gen_range(ei.min_interval..=ei.max_interval))
                .unwrap_or(base_interval); // Use spawn_rate as default
            spawn_timers.insert(entry.id.clone(), interval);
        }
        
        Self {
            car_types: cars_config.car_types.clone(),
            route: route.clone(),
            cars_config: cars_config.clone(),
            behavior_engine,
            next_car_id: 0,
            spawn_timers,
            rng,
        }
    }
    
    pub fn update(&mut self, state: &mut SimulationState) {
        // Update behavior for existing cars
        self.behavior_engine.update(state);
        
        // Handle car spawning
        self.update_spawning(state);
        
        // Handle car despawning (cars that have exited)
        self.update_despawning(state);
    }
    
    fn update_spawning(&mut self, state: &mut SimulationState) {
        // Don't spawn if we've reached the car limit
        if state.active_cars >= self.cars_config.simulation.total_cars {
            return;
        }
        
        let dt = state.dt;
        let mut spawn_requests = Vec::new();
        
        // Collect entries that need spawning
        let entries_to_check: Vec<_> = self.route.route.entries.clone();
        
        // Update spawn timers and collect spawn requests
        for (entry_id, timer) in self.spawn_timers.iter_mut() {
            *timer -= dt;
            
            if *timer <= 0.0 {
                // Try to spawn a car at this entry
                if let Some(entry) = entries_to_check.iter().find(|e| &e.id == entry_id) {
                    // Try regular spawning first, fallback to permissive spawning
                    let can_spawn = Self::can_spawn_at_entry_static(entry, state, &self.route.route.geometry) ||
                                   (state.active_cars < self.cars_config.simulation.total_cars / 2 && 
                                    Self::can_spawn_at_entry_permissive(entry, state, &self.route.route.geometry));
                    
                    if can_spawn {
                        spawn_requests.push((entry_id.clone(), entry.clone()));
                        
                        // Reset timer with random interval
                        let base_interval = 1.0 / self.cars_config.simulation.spawn_rate;
                        let entry_interval = self.cars_config.traffic_flow.entry_intervals
                            .iter()
                            .find(|ei| &ei.entry_id == entry_id);
                        
                        *timer = if let Some(interval) = entry_interval {
                            self.rng.gen_range(interval.min_interval..=interval.max_interval)
                        } else {
                            base_interval // Use spawn_rate as default
                        };
                    }
                }
            }
        }
        
        // Process spawn requests
        for (_entry_id, entry) in spawn_requests {
            self.spawn_car_at_entry(&entry, state);
        }
    }
    
    fn can_spawn_at_entry_static(
        entry: &crate::config::EntryPoint, 
        state: &SimulationState, 
        route_geom: &crate::config::RouteGeometry
    ) -> bool {
        // Calculate entry position based on geometry type
        let entry_pos = Self::calculate_entry_position(entry, route_geom);
        
        // Check if there's space at the entry point
        let min_spawn_distance = 5.0; // Minimum distance from other cars (further reduced to allow spawning in traffic)
        
        for car in &state.cars {
            let distance = (car.position - entry_pos).magnitude();
            if distance < min_spawn_distance {
                log::debug!("Cannot spawn at entry {} - car too close ({:.1}m < {:.1}m)", entry.id, distance, min_spawn_distance);
                return false;
            }
        }
        
        log::debug!("Can spawn at entry {} - no blocking cars", entry.id);
        
        true
    }
    
    fn can_spawn_at_entry_permissive(
        entry: &crate::config::EntryPoint, 
        state: &SimulationState, 
        route_geom: &crate::config::RouteGeometry
    ) -> bool {
        // Calculate entry position based on geometry type  
        let entry_pos = Self::calculate_entry_position(entry, route_geom);
        
        // Very permissive distance check - only prevent spawning if cars are extremely close
        let min_spawn_distance = 2.0; // Only 2 meters - allows spawning in tight traffic
        
        for car in &state.cars {
            let distance = (car.position - entry_pos).magnitude();
            if distance < min_spawn_distance {
                log::debug!("Cannot spawn at entry {} - car extremely close ({:.1}m < {:.1}m)", entry.id, distance, min_spawn_distance);
                return false;
            }
        }
        
        log::debug!("Can spawn at entry {} - permissive check passed", entry.id);
        
        true
    }
    
    fn spawn_car_at_entry(&mut self, entry: &crate::config::EntryPoint, state: &mut SimulationState) {
        let car_type_id = {
            let total_weight: u32 = self.car_types.iter().map(|ct| ct.weight).sum();
            let mut random_value = self.rng.gen_range(0..total_weight);
            
            let mut selected_type_id = self.car_types[0].id.clone();
            for car_type in &self.car_types {
                if random_value < car_type.weight {
                    selected_type_id = car_type.id.clone();
                    break;
                }
                random_value -= car_type.weight;
            }
            selected_type_id
        };
        
        let car_type = self.car_types.iter().find(|ct| ct.id == car_type_id).unwrap().clone();
        let behavior_name = self.behavior_engine.select_random_behavior();
        let behavior_state = self.behavior_engine.create_behavior_state(&behavior_name);
        
        let route_geom = &self.route.route.geometry;
        
        // Calculate spawn position based on geometry type
        let position = Self::calculate_entry_position(entry, route_geom);
        
        // Calculate initial velocity based on geometry type
        let (initial_velocity, heading) = Self::calculate_entry_velocity(entry, route_geom, &position);
        
        // Adaptive speed based on nearby traffic conditions
        let mut initial_speed = 26.8; // 60 mph in m/s (60 / 2.237 = 26.8) - default speed
        
        // Check nearby cars and adjust spawn speed to match traffic flow
        let check_radius = 30.0; // meters
        let mut nearby_speeds = Vec::new();
        
        for car in &state.cars {
            let distance = (car.position - position).magnitude();
            if distance < check_radius {
                nearby_speeds.push(car.velocity.magnitude());
            }
        }
        
        if !nearby_speeds.is_empty() {
            // Match average speed of nearby traffic, but ensure minimum reasonable speed
            let avg_speed = nearby_speeds.iter().sum::<f32>() / nearby_speeds.len() as f32;
            initial_speed = avg_speed.max(10.0).min(35.0); // Between 10-35 m/s (36-126 km/h)
            log::debug!("Adaptive spawn speed: {:.1} m/s based on {} nearby cars", initial_speed, nearby_speeds.len());
        }
        
        // Scale initial velocity by adaptive speed
        let velocity = initial_velocity.normalize() * initial_speed;
        let car = Car {
            id: CarId(self.next_car_id),
            position,
            velocity,
            acceleration: Vector2::zeros(),
            heading,
            length: car_type.length,
            width: car_type.width,
            max_acceleration: car_type.max_acceleration,
            max_deceleration: car_type.max_deceleration,
            preferred_speed: car_type.preferred_speed,
            current_lane: entry.lane,
            target_lane: None,
            lane_change_progress: 0.0,
            behavior: behavior_state,
            behavior_type: behavior_name,
            car_type: car_type.id.clone(),
            speed_history: [initial_speed, initial_speed, initial_speed],
            marked_for_exit: false,
            spawn_time: state.time,
            exit_time: None,
        };
        
        state.add_car(car);
        self.next_car_id += 1;
    }
    
    pub fn spawn_manual_car(&mut self, behavior_name: &str, state: &mut SimulationState) {
        // Find an available entry point
        let entry = if let Some(entry) = self.route.route.entries.first() {
            entry.clone()
        } else {
            log::warn!("No entry points available for manual car spawn");
            return;
        };
        
        // For manual spawning, be more permissive - allow spawning with closer cars
        if !Self::can_spawn_at_entry_permissive(&entry, state, &self.route.route.geometry) {
            log::debug!("Cannot spawn manual car - entry severely congested");
            return;
        }
        
        // Select a random car type
        let car_type_id = {
            let total_weight: u32 = self.car_types.iter().map(|ct| ct.weight).sum();
            let mut random_value = self.rng.gen_range(0..total_weight);
            
            let mut selected_type_id = self.car_types[0].id.clone();
            for car_type in &self.car_types {
                if random_value < car_type.weight {
                    selected_type_id = car_type.id.clone();
                    break;
                }
                random_value -= car_type.weight;
            }
            selected_type_id
        };
        
        let car_type = self.car_types.iter().find(|ct| ct.id == car_type_id).unwrap().clone();
        let behavior_state = self.behavior_engine.create_behavior_state(behavior_name);
        
        let route_geom = &self.route.route.geometry;
        
        // Calculate spawn position based on geometry type
        let position = Self::calculate_entry_position(&entry, route_geom);
        
        // Calculate initial velocity based on geometry type  
        let (initial_velocity, heading) = Self::calculate_entry_velocity(&entry, route_geom, &position);
        
        // For manual spawning, be more conservative with speed matching to ensure safety
        let mut initial_speed = 20.0; // Slightly slower default for manual spawning
        
        // Check nearby cars and adjust spawn speed to match traffic flow
        let check_radius = 25.0; // meters - smaller radius for manual spawning
        let mut nearby_speeds = Vec::new();
        
        for car in &state.cars {
            let distance = (car.position - position).magnitude();
            if distance < check_radius {
                nearby_speeds.push(car.velocity.magnitude());
            }
        }
        
        if !nearby_speeds.is_empty() {
            // For manual spawning, be more conservative - use min of nearby speeds
            let min_speed = nearby_speeds.iter().copied().fold(f32::INFINITY, f32::min);
            initial_speed = min_speed.max(5.0).min(30.0); // Between 5-30 m/s to avoid collisions
            log::debug!("Manual spawn speed: {:.1} m/s based on {} nearby cars (conservative)", initial_speed, nearby_speeds.len());
        }
        
        // Scale initial velocity by conservative speed
        let velocity = initial_velocity.normalize() * initial_speed;
        
        let car = Car {
            id: CarId(self.next_car_id),
            position,
            velocity,
            acceleration: Vector2::zeros(),
            heading,
            length: car_type.length,
            width: car_type.width,
            max_acceleration: car_type.max_acceleration,
            max_deceleration: car_type.max_deceleration,
            preferred_speed: car_type.preferred_speed,
            current_lane: entry.lane,
            target_lane: None,
            lane_change_progress: 0.0,
            behavior: behavior_state,
            behavior_type: behavior_name.to_string(),
            car_type: car_type.id.clone(),
            speed_history: [initial_speed, initial_speed, initial_speed],
            marked_for_exit: false,
            spawn_time: state.time,
            exit_time: None,
        };
        
        state.add_car(car);
        self.next_car_id += 1;
        
        log::info!("Manually spawned {} car (ID: {})", behavior_name, self.next_car_id - 1);
    }
    
    fn update_despawning(&mut self, state: &mut SimulationState) {
        let mut cars_to_remove = Vec::new();
        
        for car in &state.cars {
            // Check if car should exit at nearby exit points
            if self.should_car_exit(car) {
                cars_to_remove.push(car.id);
            }
            
            // Remove cars that have been in simulation too long (prevent buildup)
            if state.time > 600.0 { // 10 minutes
                if self.rng.gen::<f32>() < 0.001 { // 0.1% chance per frame to despawn
                    cars_to_remove.push(car.id);
                }
            }
        }
        
        for car_id in cars_to_remove {
            state.remove_car(car_id);
        }
    }
    
    fn should_car_exit(&self, car: &Car) -> bool {
        let route_geom = &self.route.route.geometry;
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        let to_car = car.position - center;
        let car_angle = to_car.y.atan2(to_car.x).to_degrees();
        
        // Normalize angle to 0-360 range
        let car_angle = if car_angle < 0.0 {
            car_angle + 360.0
        } else {
            car_angle
        };
        
        for exit in &self.route.route.exits {
            // Check if car is near an exit
            let angle_diff = (exit.angle - car_angle).abs();
            let angle_diff = if angle_diff > 180.0 {
                360.0 - angle_diff
            } else {
                angle_diff
            };
            
            // Car is near exit and in correct lane
            if angle_diff < 5.0 && car.current_lane == exit.lane {
                // Priority exit for cars marked for removal
                if car.marked_for_exit {
                    return true;
                }
                // Use behavior's exit probability for normal cars
                return true; // For simplicity, always exit when near
            }
        }
        
        false
    }
    
    fn calculate_entry_position(entry: &crate::config::EntryPoint, route_geom: &crate::config::RouteGeometry) -> Point2<f32> {
        match route_geom.geometry_type.as_str() {
            "cloverleaf" => Self::calculate_cloverleaf_entry_position(entry, route_geom),
            "donut" => Self::calculate_donut_entry_position(entry, route_geom),
            _ => {
                log::warn!("Unknown geometry type '{}', using donut spawn logic", route_geom.geometry_type);
                Self::calculate_donut_entry_position(entry, route_geom)
            }
        }
    }
    
    fn calculate_donut_entry_position(entry: &crate::config::EntryPoint, route_geom: &crate::config::RouteGeometry) -> Point2<f32> {
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        let angle_rad = entry.angle.to_radians();
        let radius = Self::get_lane_radius_static(entry.lane, route_geom);
        center + Vector2::new(
            radius * angle_rad.cos(),
            radius * angle_rad.sin()
        )
    }
    
    fn calculate_cloverleaf_entry_position(entry: &crate::config::EntryPoint, route_geom: &crate::config::RouteGeometry) -> Point2<f32> {
        // For cloverleaf, position cars based on lane numbers and highway layout
        // Lane assignments:
        // Lanes 1-3:  North-South Southbound (top to bottom) - spawn at north edge
        // Lanes 4-6:  North-South Northbound (bottom to top) - spawn at south edge
        // Lanes 7-9:  East-West Westbound (right to left) - spawn at east edge  
        // Lanes 10-12: East-West Eastbound (left to right) - spawn at west edge
        
        let highway_extent = 250.0; // How far from center to spawn
        let lane_width = route_geom.lane_width;
        
        match entry.lane {
            // North-South Southbound (lanes 1-3) - spawn at north edge
            1..=3 => {
                let lane_offset = ((entry.lane as i32) - 2) as f32 * lane_width; // -3.5, 0, 3.5
                Point2::new(lane_offset, highway_extent) // North edge
            }
            // North-South Northbound (lanes 4-6) - spawn at south edge
            4..=6 => {
                let lane_offset = (5 - (entry.lane as i32)) as f32 * lane_width; // 3.5, 0, -3.5
                Point2::new(lane_offset, -highway_extent) // South edge
            }
            // East-West Westbound (lanes 7-9) - spawn at east edge
            7..=9 => {
                let lane_offset = (8 - (entry.lane as i32)) as f32 * lane_width; // 3.5, 0, -3.5
                Point2::new(highway_extent, lane_offset) // East edge
            }
            // East-West Eastbound (lanes 10-12) - spawn at west edge
            10..=12 => {
                let lane_offset = ((entry.lane as i32) - 11) as f32 * lane_width; // -3.5, 0, 3.5
                Point2::new(-highway_extent, lane_offset) // West edge
            }
            // Invalid lane - spawn at center
            _ => {
                log::warn!("Invalid lane {} for cloverleaf, spawning at center", entry.lane);
                Point2::new(0.0, 0.0)
            }
        }
    }
    
    fn calculate_entry_velocity(entry: &crate::config::EntryPoint, route_geom: &crate::config::RouteGeometry, _position: &Point2<f32>) -> (Vector2<f32>, f32) {
        match route_geom.geometry_type.as_str() {
            "cloverleaf" => Self::calculate_cloverleaf_entry_velocity(entry),
            "donut" => Self::calculate_donut_entry_velocity(entry),
            _ => {
                log::warn!("Unknown geometry type '{}', using donut velocity logic", route_geom.geometry_type);
                Self::calculate_donut_entry_velocity(entry)
            }
        }
    }
    
    fn calculate_donut_entry_velocity(entry: &crate::config::EntryPoint) -> (Vector2<f32>, f32) {
        // For donut, calculate tangent velocity (circular motion)
        let angle_rad = entry.angle.to_radians();
        let tangent_angle = angle_rad + std::f32::consts::PI / 2.0;
        let velocity = Vector2::new(
            -tangent_angle.sin(),
            tangent_angle.cos()
        );
        (velocity, tangent_angle)
    }
    
    fn calculate_cloverleaf_entry_velocity(entry: &crate::config::EntryPoint) -> (Vector2<f32>, f32) {
        // For cloverleaf, calculate velocity based on lane assignments
        match entry.lane {
            // North-South Southbound (lanes 1-3) - heading south  
            1..=3 => (Vector2::new(0.0, -1.0), -std::f32::consts::PI / 2.0),
            // North-South Northbound (lanes 4-6) - heading north
            4..=6 => (Vector2::new(0.0, 1.0), std::f32::consts::PI / 2.0),
            // East-West Westbound (lanes 7-9) - heading west
            7..=9 => (Vector2::new(-1.0, 0.0), std::f32::consts::PI),
            // East-West Eastbound (lanes 10-12) - heading east
            10..=12 => (Vector2::new(1.0, 0.0), 0.0),
            // Invalid lane - default east
            _ => {
                log::warn!("Invalid lane {} for cloverleaf velocity, defaulting to east", entry.lane);
                (Vector2::new(1.0, 0.0), 0.0)
            }
        }
    }
    
    
    fn get_lane_radius_static(lane: u32, route_geom: &crate::config::RouteGeometry) -> f32 {
        let lane_offset = (lane as f32 - 1.0) * route_geom.lane_width;
        route_geom.inner_radius + route_geom.lane_width / 2.0 + lane_offset
    }
}