use super::{Car, Vec2, Point, SimulationState};
use crate::config::{RouteConfig, CollisionAvoidance};
use nalgebra::{Point2, Vector2};
use std::f32::consts::PI;

pub struct PhysicsEngine {
    collision_avoidance: CollisionAvoidance,
    route: RouteConfig,
}

impl PhysicsEngine {
    pub fn new(route: RouteConfig, collision_avoidance: CollisionAvoidance) -> Self {
        Self {
            collision_avoidance,
            route,
        }
    }
    
    pub fn update(&self, state: &mut SimulationState) {
        let dt = state.dt;
        
        if !state.cars.is_empty() {
            log::debug!("Physics engine updating {} cars with dt={:.3}", state.cars.len(), dt);
        }
        
        // Update car physics in parallel-safe manner
        let mut updates = Vec::with_capacity(state.cars.len());
        
        for car in &state.cars {
            log::debug!("Car {}: pos=({:.1},{:.1}) vel=({:.1},{:.1})", 
                        car.id.0, car.position.x, car.position.y, car.velocity.x, car.velocity.y);
            let update = self.calculate_car_update(car, state, dt);
            updates.push((car.id, update));
        }
        
        // Apply updates
        for (car_id, update) in updates {
            if let Some(car) = state.get_car_mut(car_id) {
                car.position = update.position;
                car.velocity = update.velocity;
                car.acceleration = update.acceleration;
                car.heading = update.heading;
                car.lane_change_progress = update.lane_change_progress;
                
                if update.lane_change_progress >= 1.0 {
                    if let Some(target_lane) = car.target_lane {
                        car.current_lane = target_lane;
                        car.target_lane = None;
                        car.lane_change_progress = 0.0;
                    }
                }
            }
        }
        
        state.time += dt;
    }
    
    fn calculate_car_update(&self, car: &Car, state: &SimulationState, dt: f32) -> CarUpdate {
        let route_geom = &self.route.route.geometry;
        
        match route_geom.geometry_type.as_str() {
            "donut" => self.calculate_donut_update(car, state, dt),
            "cloverleaf" => self.calculate_cloverleaf_update(car, state, dt),
            _ => {
                // Default to donut behavior
                self.calculate_donut_update(car, state, dt)
            }
        }
    }
    
    fn calculate_donut_update(&self, car: &Car, state: &SimulationState, dt: f32) -> CarUpdate {
        let route_geom = &self.route.route.geometry;
        
        // Get current position on donut
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        let to_car = car.position - center;
        let current_angle = to_car.y.atan2(to_car.x);
        let current_radius = to_car.magnitude();
        
        // Calculate target lane position
        let target_radius = self.get_target_radius(car, route_geom);
        
        // Find nearest cars for collision avoidance
        let (front_car, front_distance) = self.find_front_car(car, state);
        let following_distance = self.calculate_following_distance(car);
        
        // Calculate desired speed based on traffic and behavior
        let mut target_speed = car.behavior.target_speed;
        
        // Collision avoidance
        if let Some(distance) = front_distance {
            if distance < self.collision_avoidance.emergency_brake_distance {
                target_speed = 0.0; // Emergency brake
            } else if distance < self.collision_avoidance.warning_distance {
                let brake_factor = (distance - self.collision_avoidance.emergency_brake_distance) 
                    / (self.collision_avoidance.warning_distance - self.collision_avoidance.emergency_brake_distance);
                target_speed *= brake_factor;
            } else if distance < following_distance {
                // Maintain following distance
                if let Some(front_car) = front_car {
                    target_speed = front_car.velocity.magnitude().min(target_speed);
                }
            }
        }
        
        // Calculate acceleration
        let current_speed = car.velocity.magnitude();
        let speed_diff = target_speed - current_speed;
        let _acceleration_magnitude = if speed_diff > 0.0 {
            (speed_diff / dt).min(car.max_acceleration)
        } else {
            (speed_diff / dt).max(-car.max_deceleration)
        };
        
        // Calculate new heading (tangent to circle)
        let tangent_angle = current_angle + PI / 2.0;
        let heading = if car.velocity.magnitude() > 0.1 {
            car.velocity.y.atan2(car.velocity.x)
        } else {
            tangent_angle
        };
        
        // Update lane change progress
        let mut lane_change_progress = car.lane_change_progress;
        if car.target_lane.is_some() {
            lane_change_progress += dt / self.route.route.traffic_rules.lane_change_time;
            lane_change_progress = lane_change_progress.min(1.0);
        }
        
        // Calculate position based on current and target radius
        let _lerp_radius = if car.target_lane.is_some() {
            let current_target_radius = self.get_lane_radius(car.current_lane, route_geom);
            let target_target_radius = self.get_lane_radius(car.target_lane.unwrap(), route_geom);
            current_target_radius + (target_target_radius - current_target_radius) * lane_change_progress
        } else {
            target_radius
        };
        
        // Calculate velocity (tangential + radial components)
        let tangential_speed = target_speed;
        // For counter-clockwise motion around the circle
        let tangent_dir = Vector2::new(-tangent_angle.sin(), tangent_angle.cos());
        
        // Add radial component for lane changes
        let radial_component = if car.target_lane.is_some() {
            let radial_speed = (target_radius - current_radius) / self.route.route.traffic_rules.lane_change_time;
            let radial_dir = to_car.normalize();
            radial_dir * radial_speed
        } else {
            Vector2::zeros()
        };
        
        let new_velocity = tangent_dir * tangential_speed + radial_component;
        
        // Update position using angular motion for circular path
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        
        // Calculate angular velocity from tangential speed
        let angular_velocity = tangential_speed / target_radius;
        let new_angle = current_angle + angular_velocity * dt;
        
        // Calculate new position on the circle
        let new_position = center + target_radius * Vector2::new(new_angle.cos(), new_angle.sin());
        
        // Calculate acceleration vector
        let acceleration = if dt > 0.0 {
            (new_velocity - car.velocity) / dt
        } else {
            Vector2::zeros()
        };
        
        CarUpdate {
            position: new_position,
            velocity: new_velocity,
            acceleration,
            heading,
            lane_change_progress,
        }
    }
    
    fn calculate_cloverleaf_update(&self, car: &Car, state: &SimulationState, dt: f32) -> CarUpdate {
        // Proper cloverleaf implementation with highway paths and loop ramps
        
        // Find nearest cars for collision avoidance
        let (front_car, front_distance) = self.find_front_car_straight(car, state);
        let following_distance = self.calculate_following_distance(car);
        
        // Calculate desired speed based on traffic and behavior
        let mut target_speed = car.behavior.target_speed;
        
        // Collision avoidance
        if let Some(distance) = front_distance {
            if distance < self.collision_avoidance.emergency_brake_distance {
                target_speed = 0.0; // Emergency brake
            } else if distance < self.collision_avoidance.warning_distance {
                let brake_factor = (distance - self.collision_avoidance.emergency_brake_distance) 
                    / (self.collision_avoidance.warning_distance - self.collision_avoidance.emergency_brake_distance);
                target_speed *= brake_factor;
            } else if distance < following_distance {
                // Maintain following distance
                if let Some(front_car) = front_car {
                    target_speed = front_car.velocity.magnitude().min(target_speed);
                }
            }
        }
        
        // Determine path type based on lane number
        let (path_direction, new_position, new_velocity, heading) = self.calculate_cloverleaf_path(car, target_speed, dt);
        
        // Calculate acceleration vector
        let acceleration = if dt > 0.0 {
            (new_velocity - car.velocity) / dt
        } else {
            Vector2::zeros()
        };
        
        CarUpdate {
            position: new_position,
            velocity: new_velocity,
            acceleration,
            heading,
            lane_change_progress: car.lane_change_progress,
        }
    }
    
    fn calculate_cloverleaf_path(&self, car: &Car, target_speed: f32, dt: f32) -> (String, Point, Vector2<f32>, f32) {
        // Lane assignments for cloverleaf:
        // Lanes 1-3:  North-South Southbound (top to bottom)
        // Lanes 4-6:  North-South Northbound (bottom to top)  
        // Lanes 7-9:  East-West Westbound (right to left)
        // Lanes 10-12: East-West Eastbound (left to right)
        
        let route_geom = &self.route.route.geometry;
        let highway_half_width = route_geom.highway_width.unwrap_or(40.0) / 2.0;
        
        match car.current_lane {
            // North-South Southbound (lanes 1-3)
            1..=3 => {
                let lane_offset = (car.current_lane - 2) as f32 * route_geom.lane_width; // -3.5, 0, 3.5
                let x_pos = lane_offset;
                let y_pos = car.position.y - target_speed * dt;
                let heading = -std::f32::consts::PI / 2.0; // Pointing south
                let velocity = Vector2::new(0.0, -target_speed);
                
                ("southbound".to_string(), Point2::new(x_pos, y_pos), velocity, heading)
            }
            // North-South Northbound (lanes 4-6)
            4..=6 => {
                let lane_offset = (5 - car.current_lane) as f32 * route_geom.lane_width; // 3.5, 0, -3.5
                let x_pos = lane_offset;
                let y_pos = car.position.y + target_speed * dt;
                let heading = std::f32::consts::PI / 2.0; // Pointing north
                let velocity = Vector2::new(0.0, target_speed);
                
                ("northbound".to_string(), Point2::new(x_pos, y_pos), velocity, heading)
            }
            // East-West Westbound (lanes 7-9)
            7..=9 => {
                let lane_offset = (8 - car.current_lane) as f32 * route_geom.lane_width; // 3.5, 0, -3.5
                let y_pos = lane_offset;
                let x_pos = car.position.x - target_speed * dt;
                let heading = std::f32::consts::PI; // Pointing west
                let velocity = Vector2::new(-target_speed, 0.0);
                
                ("westbound".to_string(), Point2::new(x_pos, y_pos), velocity, heading)
            }
            // East-West Eastbound (lanes 10-12)
            10..=12 => {
                let lane_offset = (car.current_lane - 11) as f32 * route_geom.lane_width; // -3.5, 0, 3.5
                let y_pos = lane_offset;
                let x_pos = car.position.x + target_speed * dt;
                let heading = 0.0; // Pointing east
                let velocity = Vector2::new(target_speed, 0.0);
                
                ("eastbound".to_string(), Point2::new(x_pos, y_pos), velocity, heading)
            }
            // Loop ramps or invalid lanes - maintain current direction
            _ => {
                let current_heading = if car.velocity.magnitude() > 0.1 {
                    car.velocity.y.atan2(car.velocity.x)
                } else {
                    0.0
                };
                let velocity_direction = Vector2::new(current_heading.cos(), current_heading.sin());
                let new_velocity = velocity_direction * target_speed;
                let new_position = car.position + new_velocity * dt;
                
                ("loop".to_string(), new_position, new_velocity, current_heading)
            }
        }
    }
    
    fn find_front_car_straight<'a>(&self, car: &Car, state: &'a SimulationState) -> (Option<&'a Car>, Option<f32>) {
        // Simplified straight-line front car detection for cloverleaf
        let car_direction = if car.velocity.magnitude() > 0.1 {
            car.velocity.normalize()
        } else {
            Vector2::new(1.0, 0.0) // Default to eastward
        };
        
        let mut closest_car: Option<&Car> = None;
        let mut closest_distance = f32::INFINITY;
        
        for other_car in &state.cars {
            if other_car.id == car.id {
                continue;
            }
            
            // Only consider cars in same lane
            if other_car.current_lane != car.current_lane {
                continue;
            }
            
            let to_other = other_car.position - car.position;
            let distance = to_other.magnitude();
            
            // Check if other car is in front (dot product > 0)
            if to_other.dot(&car_direction) > 0.0 && distance < closest_distance {
                closest_distance = distance;
                closest_car = Some(other_car);
            }
        }
        
        if closest_distance == f32::INFINITY {
            (None, None)
        } else {
            (closest_car, Some(closest_distance))
        }
    }
    
    fn get_target_radius(&self, car: &Car, route_geom: &crate::config::RouteGeometry) -> f32 {
        if let Some(target_lane) = car.target_lane {
            self.get_lane_radius(target_lane, route_geom)
        } else {
            self.get_lane_radius(car.current_lane, route_geom)
        }
    }
    
    fn get_lane_radius(&self, lane: u32, route_geom: &crate::config::RouteGeometry) -> f32 {
        let lane_offset = (lane as f32 - 1.0) * route_geom.lane_width;
        route_geom.inner_radius + route_geom.lane_width / 2.0 + lane_offset
    }
    
    fn find_front_car<'a>(&self, car: &Car, state: &'a SimulationState) -> (Option<&'a Car>, Option<f32>) {
        let route_geom = &self.route.route.geometry;
        let center = Point2::new(route_geom.center_x, route_geom.center_y);
        let to_car = car.position - center;
        let car_angle = to_car.y.atan2(to_car.x);
        
        let mut closest_car: Option<&Car> = None;
        let mut closest_distance = f32::INFINITY;
        
        for other_car in &state.cars {
            if other_car.id == car.id {
                continue;
            }
            
            // Only consider cars in same lane or target lane
            if other_car.current_lane != car.current_lane && 
               Some(other_car.current_lane) != car.target_lane {
                continue;
            }
            
            let to_other = other_car.position - center;
            let other_angle = to_other.y.atan2(to_other.x);
            
            // Calculate angular distance (accounting for wrap-around)
            let mut angle_diff = other_angle - car_angle;
            if angle_diff < 0.0 {
                angle_diff += 2.0 * PI;
            }
            
            // Only consider cars in front
            if angle_diff > 0.0 && angle_diff < PI {
                let arc_distance = angle_diff * to_car.magnitude();
                if arc_distance < closest_distance {
                    closest_distance = arc_distance;
                    closest_car = Some(other_car);
                }
            }
        }
        
        if closest_distance == f32::INFINITY {
            (None, None)
        } else {
            (closest_car, Some(closest_distance))
        }
    }
    
    fn calculate_following_distance(&self, car: &Car) -> f32 {
        let base_distance = self.route.route.traffic_rules.following_distance * car.velocity.magnitude();
        base_distance * car.behavior.following_distance_factor + self.collision_avoidance.safety_margin
    }
}

#[derive(Debug, Clone)]
struct CarUpdate {
    position: Point,
    velocity: Vec2,
    acceleration: Vec2,
    heading: f32,
    lane_change_progress: f32,
}