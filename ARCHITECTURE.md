# Traffic Simulator Architecture

## Overview

This traffic simulator is designed for high-performance GPU-accelerated simulation with interactive 2D visualization. It supports both CPU and OpenCL GPU compute for simulation physics, with real-time rendering using modern Rust graphics libraries.

## Technology Stack

### Graphics & Rendering
- **wgpu**: Cross-platform GPU graphics API (Vulkan/Metal/DirectX 12/OpenGL ES)
- **Vello**: GPU compute-centric 2D vector renderer for smooth zoom/pan
- **winit**: Cross-platform window creation and event handling
- **kurbo**: 2D curves and shapes library

### GPU Compute
- **opencl3**: Primary OpenCL 3.0 bindings for GPU acceleration
- **ocl**: Alternative mature OpenCL wrapper (optional feature)
- CPU fallback for systems without OpenCL support

### Configuration & Data
- **TOML**: Human-readable configuration files
- **serde**: Serialization/deserialization
- **nalgebra**: Linear algebra and mathematics
- **rand**: Random number generation for traffic patterns

## Architecture Components

### 1. Simulation Engine (`src/simulation/`)
- **Physics Engine**: Car movement, collision detection, lane changes
- **Traffic Manager**: Spawning, despawning, route following
- **Behavior System**: Driver personality implementation
- **Performance Monitor**: CPU/GPU timing measurements

### 2. Rendering System (`src/graphics/`)
- **Viewport**: Zoom/pan camera with smooth transitions
- **Car Renderer**: Efficient batched vehicle rendering
- **Route Renderer**: Road geometry and lane markings
- **UI Overlay**: Performance metrics, controls

### 3. Configuration System (`src/config/`)
- **Route Loader**: Parse route.toml files
- **Car Config**: Load car types and behaviors from cars.toml
- **Validation**: Ensure configuration correctness

### 4. GPU Compute (`src/compute/`)
- **OpenCL Kernels**: Parallel physics calculations
- **Buffer Management**: GPU memory optimization
- **CPU Fallback**: Pure Rust implementation for compatibility

## File Format Documentation

### Route Configuration (`route.toml`)

Defines the road geometry, entry/exit points, and traffic rules.

#### Structure:
```toml
[route]
name = "Route Name"
description = "Route description"

[route.geometry]
type = "donut"  # Currently supports "donut" shape
center_x = 0.0  # Route center X coordinate
center_y = 0.0  # Route center Y coordinate
inner_radius = 150.0    # Inner radius (meters)
outer_radius = 200.0    # Outer radius (meters)
lane_width = 3.5        # Width per lane (meters)
lane_count = 3          # Lanes in each direction

[[route.entries]]       # Array of entry points
id = "unique_id"        # Entry identifier
type = "interior"       # "interior" or "exterior"
angle = 0.0            # Angle in degrees (0=right, 90=top)
position = "inner"      # "inner" or "outer" relative to donut
lane = 1               # Target lane (1-based)
merge_distance = 50.0   # Distance to complete merge (meters)

[[route.exits]]         # Array of exit points
id = "unique_id"        # Exit identifier  
type = "exterior"       # "interior" or "exterior"
angle = 90.0           # Angle in degrees
position = "outer"      # "inner" or "outer" relative to donut
lane = 3               # Source lane (1-based)
exit_distance = 75.0    # Deceleration lane length (meters)

[route.traffic_rules]
speed_limit = 27.8      # Maximum speed (m/s)
min_speed = 13.9        # Minimum speed (m/s)
following_distance = 2.0 # Base following time (seconds)
lane_change_time = 3.0   # Time to complete lane change (seconds)

[route.surface]
friction_coefficient = 0.7  # Road surface friction
banking_angle = 2.0         # Banking angle for curves (degrees)
```

### Car Configuration (`cars.toml`)

Defines vehicle types, driver behaviors, and simulation parameters.

#### Structure:
```toml
[simulation]
total_cars = 100        # Maximum cars in simulation
spawn_rate = 0.5        # Cars spawned per second
simulation_duration = 300.0  # Total simulation time (seconds)

[[car_types]]           # Array of vehicle types
id = "sedan"            # Unique car type identifier
weight = 30             # Percentage of traffic (must sum to 100)
length = 4.5            # Vehicle length (meters)
width = 1.8             # Vehicle width (meters)
max_acceleration = 3.0   # Maximum acceleration (m/s²)
max_deceleration = 8.0   # Maximum braking (m/s²)
preferred_speed = 25.0   # Preferred cruising speed (m/s)

[behavior.normal]       # Driver behavior patterns
name = "Normal Driver"  # Human-readable name
weight = 60            # Percentage of drivers
following_distance_factor = 1.0  # Multiplier for base following distance
lane_change_frequency = 0.8      # Lane changes per minute
speed_variance = 1.0             # Speed preference multiplier
reaction_time = 1.2              # Driver reaction time (seconds)
exit_probability = 0.25          # Probability of taking available exit

[collision_avoidance]
safety_margin = 1.5            # Extra spacing buffer (meters)
emergency_brake_distance = 20.0 # Emergency braking threshold (meters)
warning_distance = 40.0         # Slow-down warning distance (meters)
lateral_safety_margin = 0.5     # Lane change safety margin (meters)

[performance]
enable_gpu_timing = true    # Enable GPU performance monitoring
enable_cpu_timing = true    # Enable CPU performance monitoring
timing_samples = 100        # Frames to average for timing display
```

## Performance Features

### GPU Acceleration
- **Parallel Physics**: OpenCL kernels for collision detection and movement
- **Batched Rendering**: Efficient GPU-based 2D graphics with Vello
- **Memory Optimization**: Minimize CPU-GPU transfers

### CPU Fallback
- Pure Rust implementation for systems without OpenCL
- Automatic detection and graceful fallback
- Comparable accuracy with different performance characteristics

### Real-time Monitoring
- Frame timing display
- Simulation step duration
- GPU/CPU load indicators
- Memory usage statistics

## Controls & Interaction

### Viewport Controls
- **Mouse Wheel**: Zoom in/out
- **Mouse Drag**: Pan viewport
- **Keyboard Arrows**: Precise camera movement
- **Home Key**: Reset to default view

### Simulation Controls
- **Space**: Pause/Resume simulation
- **+/-**: Increase/Decrease simulation speed
- **R**: Reset simulation
- **S**: Single step (when paused)

### Performance Toggles
- **F1**: Toggle performance overlay
- **F2**: Switch between CPU/GPU compute
- **F3**: Toggle vsync
- **F4**: Toggle debug rendering

## Extension Points

### Custom Route Types
Add new geometry types by implementing the `RouteGeometry` trait:
```rust
pub trait RouteGeometry {
    fn get_position_at(&self, progress: f32) -> Vec2;
    fn get_lane_center(&self, lane: u32, progress: f32) -> Vec2;
    fn get_exits_in_range(&self, start: f32, end: f32) -> Vec<Exit>;
}
```

### Custom Behaviors
Define new driver behaviors by implementing the `DriverBehavior` trait:
```rust
pub trait DriverBehavior {
    fn update_target_speed(&mut self, context: &TrafficContext) -> f32;
    fn should_change_lanes(&mut self, context: &TrafficContext) -> Option<LaneChange>;
    fn should_take_exit(&mut self, exit: &Exit, context: &TrafficContext) -> bool;
}
```

### Custom Rendering
Add visual enhancements through the rendering pipeline:
- Custom car sprites/models
- Traffic density heatmaps  
- Lane utilization visualization
- Real-time performance graphs

This architecture provides a solid foundation for a high-performance traffic simulator with extensive customization capabilities.