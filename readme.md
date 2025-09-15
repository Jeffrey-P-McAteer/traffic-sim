# Traffic Simulator

A high-performance, GPU-accelerated traffic simulation with interactive 2D visualization built in Rust. Features real-time physics, multiple driving behaviors, and support for different road geometries including highways and cloverleaf interchanges.

![Traffic Simulation Demo](demo-01-cloverleaf.mp4)

## Features

- **GPU-Accelerated Computing**: OpenCL support for parallel physics calculations with CPU fallback
- **Real-Time Visualization**: Hardware-accelerated 2D graphics using wgpu and Vello
- **Advanced Physics**: Realistic car movement, collision avoidance, and traffic flow
- **Multiple Route Types**: Support for circular highways (donut) and cloverleaf interchanges
- **Diverse Driving Behaviors**: Aggressive, normal, cautious, erratic, and strategic driver personalities
- **Interactive Controls**: Real-time simulation control, camera movement, and manual car spawning
- **Performance Monitoring**: Built-in FPS tracking and performance metrics
- **Configurable**: Extensive TOML-based configuration for routes, cars, and behaviors

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- OpenCL drivers (optional, for GPU acceleration)
- Modern graphics driver supporting Vulkan/Metal/DirectX 12

### Installation

```bash
git clone <repository-url>
cd traffic-sim
cargo build --release
```

### Running the Simulation

```bash
# Run with default configuration (donut highway)
cargo run --release

# Run cloverleaf interchange simulation
cargo run --release -- --route route2.toml

# Force CPU backend
cargo run --release -- --backend cpu

# Enable verbose logging
cargo run --release -- --verbose

# Use custom configurations
cargo run --release -- --route my_route.toml --cars my_cars.toml
```

### Basic Controls

- **Space**: Pause/Resume simulation
- **R**: Reset simulation
- **1-9**: Set simulation speed (1x to 9x)
- **ESC**: Exit simulation
- **Mouse Wheel**: Zoom in/out
- **Mouse Drag**: Pan viewport

### Manual Car Controls

- **A**: Spawn aggressive driver (Shift+A to remove)
- **N**: Spawn normal driver (Shift+N to remove)
- **C**: Spawn cautious driver (Shift+C to remove)
- **E**: Spawn erratic driver (Shift+E to remove)
- **S**: Spawn strategic driver (Shift+S to remove)

## Architecture Overview

The simulator is built with a modular architecture designed for high performance and extensibility:

### Core Systems

#### 1. **Simulation Engine** (`src/simulation/`)
- **Physics Engine**: Handles car movement, collision detection, and lane changes
- **Traffic Manager**: Manages car spawning, despawning, and route following
- **Behavior System**: Implements different driver personalities and decision-making
- **Performance Tracker**: Monitors frame rates and simulation performance

#### 2. **Graphics System** (`src/graphics/`)
- **Renderer**: GPU-accelerated 2D rendering using Vello vector graphics
- **Viewport**: Interactive camera with smooth zoom and pan
- **UI System**: Real-time performance overlay and controls

#### 3. **Compute Backend** (`src/compute/`)
- **GPU Backend**: OpenCL-accelerated parallel physics calculations
- **CPU Backend**: Pure Rust fallback for systems without OpenCL
- **Automatic Detection**: Graceful fallback when GPU compute is unavailable

#### 4. **Configuration System** (`src/config/`)
- **Route Configuration**: TOML-based route geometry and traffic rules
- **Car Configuration**: Vehicle types, behaviors, and simulation parameters
- **Validation**: Ensures configuration correctness and provides helpful errors

## Configuration

### Route Configuration (`route.toml`)

Define road geometry, entry/exit points, and traffic rules:

```toml
[route]
name = "Highway Donut"
description = "Circular highway with entries and exits"

[route.geometry]
type = "donut"                  # "donut" or "cloverleaf"
center_x = 0.0
center_y = 0.0
inner_radius = 150.0            # meters
outer_radius = 200.0            # meters
lane_width = 3.5                # meters per lane
lane_count = 3                  # lanes in each direction

[[route.entries]]
id = "entry_1"
type = "interior"
angle = 0.0                     # degrees (0=right, 90=top)
lane = 1                        # target lane (1-based)
merge_distance = 50.0           # meters

[route.traffic_rules]
speed_limit = 27.8              # m/s (100 km/h)
min_speed = 13.9                # m/s (50 km/h)
following_distance = 2.0        # seconds
lane_change_time = 3.0          # seconds
```

### Car Configuration (`cars.toml`)

Define vehicle types, driver behaviors, and simulation parameters:

```toml
[simulation]
total_cars = 100                # maximum cars in simulation
spawn_rate = 2.0                # cars per second
simulation_duration = 300.0     # seconds

[[car_types]]
id = "sedan"
weight = 30                     # percentage of traffic
length = 4.5                    # meters
width = 1.8                     # meters
max_acceleration = 3.0          # m/s²
max_deceleration = 8.0          # m/s²
preferred_speed = 25.0          # m/s

[behavior.aggressive]
name = "Aggressive Driver"
weight = 15                     # percentage of drivers
following_distance_factor = 0.7 # closer following
lane_change_frequency = 2.0     # changes per minute
speed_variance = 1.15           # 15% faster than preferred
reaction_time = 0.8             # seconds
exit_probability = 0.15         # lower exit probability
```

## Route Types

### Donut Highway
A circular highway with interior entrances and exterior exits. Cars follow a curved path with:
- Configurable radius and lane count
- Interior merge points for entering traffic
- Exterior exit points for leaving traffic
- Realistic circular motion physics

### Cloverleaf Interchange
A complex four-way highway interchange featuring:
- Two intersecting highways (North-South and East-West)
- Loop ramps for left-turn movements
- Through traffic lanes for straight movements
- Realistic highway merging and lane changes
- 12 total lanes (3 per direction × 4 directions)

## Driver Behaviors

### Aggressive Drivers (15% of traffic)
- Faster speeds (15% above preferred)
- Closer following distances
- Frequent lane changes (2.0 per minute)
- Quick reaction times (0.8 seconds)
- Lower exit probability

### Normal Drivers (50% of traffic)
- Standard speeds and following distances
- Moderate lane changes (0.8 per minute)
- Average reaction times (1.2 seconds)
- Balanced exit probability

### Cautious Drivers (20% of traffic)
- Slower speeds (15% below preferred)
- Larger following distances
- Infrequent lane changes (0.3 per minute)
- Quick reactions but conservative behavior
- Higher exit probability

### Erratic Drivers (5% of traffic)
- Unpredictable speeds (20% variance)
- Inconsistent following distances
- Very frequent lane changes (3.0 per minute)
- Slower reaction times (1.5 seconds)
- High exit probability

### Strategic Drivers (10% of traffic)
- Optimal speeds (5% above preferred)
- Calculated following distances
- Strategic lane changes (1.2 per minute)
- Quick reaction times
- Lower exit probability
- Traffic-aware behavior (avoids slowdowns)

## Performance Features

### GPU Acceleration
- **OpenCL Computing**: Parallel physics calculations for hundreds of cars
- **Automatic Fallback**: Graceful degradation to CPU when GPU unavailable
- **Memory Optimization**: Efficient GPU memory management

### Optimized Rendering
- **Vector Graphics**: Smooth scaling with Vello 2D renderer
- **Hardware Acceleration**: GPU-accelerated graphics pipeline
- **Batched Rendering**: Efficient car and road rendering

### Real-Time Monitoring
- **Performance Metrics**: Frame time, simulation time, CPU/GPU usage
- **Configurable Tracking**: Adjustable sampling windows
- **Visual Feedback**: On-screen performance display

## Command Line Options

```bash
USAGE:
    traffic-sim [OPTIONS]

OPTIONS:
    -b, --backend <BACKEND>    Simulation backend [default: cpu] [possible values: cpu, gpu]
    -r, --route <ROUTE>        Route configuration file [default: route.toml]
    -c, --cars <CARS>          Cars configuration file [default: cars.toml]
    -s, --seed <SEED>          Random seed for reproducible simulations
    -v, --verbose              Enable verbose logging
        --font-size <SIZE>     UI font size [default: 14.0]
    -h, --help                 Print help information
```

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (recommended for performance)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Performance Testing

```bash
# Run performance benchmarks
cargo bench --bench simulation_performance

# Test with different configurations
cargo run --release -- --verbose --backend gpu
cargo run --release -- --route route2.toml --cars cars.toml
```

### Code Structure

```
src/
├── main.rs                 # Application entry point and main loop
├── lib.rs                  # Library exports
├── config/                 # Configuration loading and validation
│   ├── mod.rs
│   ├── cars.rs            # Car and behavior configuration
│   └── route.rs           # Route geometry and traffic rules
├── simulation/             # Core simulation logic
│   ├── mod.rs             # Simulation state and data structures
│   ├── physics.rs         # Physics engine and car movement
│   ├── behavior.rs        # Driver behavior system
│   └── traffic.rs         # Traffic management and spawning
├── graphics/               # Rendering and visualization
│   ├── mod.rs
│   ├── renderer.rs        # 2D graphics rendering
│   ├── ui.rs              # User interface overlay
│   └── viewport.rs        # Camera and viewport controls
└── compute/                # Compute backends
    ├── mod.rs
    ├── cpu.rs             # CPU simulation backend
    └── gpu.rs             # OpenCL GPU backend
```

## System Requirements

### Minimum Requirements
- **OS**: Windows 10, macOS 10.15, or Linux with X11/Wayland
- **CPU**: 2-core processor, 2.0 GHz
- **Memory**: 4 GB RAM
- **Graphics**: OpenGL 3.3 or DirectX 11 support

### Recommended Requirements
- **OS**: Windows 11, macOS 12+, or recent Linux distribution
- **CPU**: 4+ core processor, 3.0+ GHz
- **Memory**: 8+ GB RAM
- **Graphics**: Discrete GPU with Vulkan/Metal/DirectX 12 support
- **OpenCL**: OpenCL 1.2+ for GPU acceleration

## Troubleshooting

### OpenCL Issues
If GPU backend fails to initialize:
```bash
# Check OpenCL availability
clinfo

# Force CPU backend
cargo run --release -- --backend cpu
```

### Performance Issues
For low frame rates:
- Reduce total car count in `cars.toml`
- Lower spawn rate
- Use CPU backend if GPU backend is unstable
- Close other graphics-intensive applications

### Configuration Errors
The simulator validates configuration files on startup and provides detailed error messages for:
- Invalid geometry parameters
- Inconsistent lane assignments
- Missing required fields
- Out-of-range values

## License

This project is licensed under the MIT OR Apache-2.0 license. See the LICENSE files for details.

## Contributing

Contributions are welcome! Please see CONTRIBUTING.md for development guidelines and code standards.

## Acknowledgments

Built with these excellent Rust libraries:
- [wgpu](https://github.com/gfx-rs/wgpu) - Cross-platform graphics
- [vello](https://github.com/linebender/vello) - 2D vector graphics
- [winit](https://github.com/rust-windowing/winit) - Window management
- [nalgebra](https://github.com/dimforge/nalgebra) - Linear algebra
- [opencl3](https://github.com/kenba/opencl3) - OpenCL bindings
- [egui](https://github.com/emilk/egui) - Immediate mode GUI