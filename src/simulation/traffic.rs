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
                    if Self::can_spawn_at_entry_static(entry, state, &self.route.route.geometry) {
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
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        
        // Calculate entry position
        let angle_rad = entry.angle.to_radians();
        let radius = Self::get_lane_radius_static(entry.lane, route_geom);
        let entry_pos = center + Vector2::new(
            radius * angle_rad.cos(),
            radius * angle_rad.sin()
        );
        
        // Check if there's space at the entry point
        let min_spawn_distance = 10.0; // Minimum distance from other cars (reduced from 20.0)
        
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
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        
        // Calculate spawn position
        let angle_rad = entry.angle.to_radians();
        let radius = Self::get_lane_radius_static(entry.lane, route_geom);
        let position = center + Vector2::new(
            radius * angle_rad.cos(),
            radius * angle_rad.sin()
        );
        
        // Calculate initial velocity (tangent to circle)
        let tangent_angle = angle_rad + std::f32::consts::PI / 2.0;
        let initial_speed = car_type.preferred_speed * 0.5; // Start at half speed
        let velocity = Vector2::new(
            -tangent_angle.sin() * initial_speed,
            tangent_angle.cos() * initial_speed
        );
        
        let car = Car {
            id: CarId(self.next_car_id),
            position,
            velocity,
            acceleration: Vector2::zeros(),
            heading: tangent_angle,
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
        };
        
        state.add_car(car);
        self.next_car_id += 1;
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
                // Use behavior's exit probability
                return true; // For simplicity, always exit when near
            }
        }
        
        false
    }
    
    
    fn get_lane_radius_static(lane: u32, route_geom: &crate::config::RouteGeometry) -> f32 {
        let lane_offset = (lane as f32 - 1.0) * route_geom.lane_width;
        route_geom.inner_radius + route_geom.lane_width / 2.0 + lane_offset
    }
}