use criterion::{black_box, criterion_group, criterion_main, Criterion};
use traffic_sim::{
    config::SimulationConfig,
    simulation::SimulationState,
    compute::{ComputeBackend, SimulationBackend},
};

fn benchmark_cpu_simulation(c: &mut Criterion) {
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")
        .expect("Failed to load configuration");
    
    let mut backend = ComputeBackend::new_cpu(
        config.cars.clone(),
        config.route.clone(),
        Some(42)
    );
    
    let mut state = SimulationState::new(1.0 / 60.0);
    
    // Pre-populate with some cars for realistic benchmarking
    for _ in 0..50 {
        backend.update(&mut state).unwrap();
    }
    
    c.bench_function("cpu_simulation_update", |b| {
        b.iter(|| {
            backend.update(black_box(&mut state)).unwrap();
        })
    });
}

#[cfg(feature = "gpu-sim")]
fn benchmark_gpu_simulation(c: &mut Criterion) {
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")
        .expect("Failed to load configuration");
    
    if let Ok(mut backend) = ComputeBackend::new_gpu(
        config.cars.clone(),
        config.route.clone(),
        Some(42)
    ) {
        let mut state = SimulationState::new(1.0 / 60.0);
        
        // Pre-populate with some cars for realistic benchmarking
        for _ in 0..50 {
            backend.update(&mut state).unwrap();
        }
        
        c.bench_function("gpu_simulation_update", |b| {
            b.iter(|| {
                backend.update(black_box(&mut state)).unwrap();
            })
        });
    }
}

fn benchmark_simulation_scaling(c: &mut Criterion) {
    let config = SimulationConfig::load_from_files("route.toml", "cars.toml")
        .expect("Failed to load configuration");
    
    let mut group = c.benchmark_group("simulation_scaling");
    
    for car_count in [10, 50, 100, 200].iter() {
        let mut backend = ComputeBackend::new_cpu(
            config.cars.clone(),
            config.route.clone(),
            Some(42)
        );
        
        let mut state = SimulationState::new(1.0 / 60.0);
        
        // Pre-populate with specified number of cars
        for _ in 0..*car_count {
            backend.update(&mut state).unwrap();
            if state.active_cars >= *car_count {
                break;
            }
        }
        
        group.bench_with_input(
            format!("cpu_{}_cars", car_count),
            car_count,
            |b, _car_count| {
                b.iter(|| {
                    backend.update(black_box(&mut state)).unwrap();
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches, 
    benchmark_cpu_simulation,
    #[cfg(feature = "gpu-sim")]
    benchmark_gpu_simulation,
    benchmark_simulation_scaling
);
criterion_main!(benches);