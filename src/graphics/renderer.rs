use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::window::Window;
use crate::simulation::{SimulationState, Car};
use nalgebra::Matrix4;

pub struct TrafficRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    
    // Rendering pipeline
    render_pipeline: wgpu::RenderPipeline,
    
    // Uniform buffers
    view_bind_group: wgpu::BindGroup,
    view_buffer: wgpu::Buffer,
    
    // Vertex data
    car_vertex_buffer: wgpu::Buffer,
    road_vertex_buffer: wgpu::Buffer,
    road_vertex_count: u32,
    car_instance_buffer: wgpu::Buffer,
    road_identity_instance_buffer: wgpu::Buffer,
    
    // Shader layouts
    view_bind_group_layout: wgpu::BindGroupLayout,
    
    max_cars: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CarInstance {
    transform: [[f32; 4]; 4],
    color: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniforms {
    view_proj: [[f32; 4]; 4],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl CarInstance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<CarInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Transform matrix (4 vec4s)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (2 * mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (3 * mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: (4 * mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const SHADER_SOURCE: &str = r#"
struct ViewUniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> view: ViewUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    
    var out: VertexOutput;
    out.color = model.color * instance.color;
    out.clip_position = view.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

impl TrafficRenderer {
    pub async fn new(window: std::sync::Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        
        // Create surface
        let surface = instance.create_surface(window.clone())?;
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find an appropriate adapter"))?;
        
        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Traffic Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        
        // Create bind group layout for view uniforms
        let view_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("view_bind_group_layout"),
        });
        
        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&view_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), CarInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        // Create buffers
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("View Buffer"),
            size: std::mem::size_of::<ViewUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_buffer.as_entire_binding(),
            }],
            label: Some("view_bind_group"),
        });
        
        // Create vertex buffers
        let car_vertices = Self::create_car_vertices();
        let car_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Car Vertex Buffer"),
            contents: bytemuck::cast_slice(&car_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let road_vertices = Self::create_road_vertices();
        let road_vertex_count = road_vertices.len() as u32;
        let road_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Road Vertex Buffer"),
            contents: bytemuck::cast_slice(&road_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let max_cars = 1000;
        let car_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Car Instance Buffer"),
            size: (std::mem::size_of::<CarInstance>() * max_cars) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create identity instance buffer for road rendering (since roads don't need per-instance transforms)
        let identity_transform = Matrix4::identity();
        let identity_instance = CarInstance {
            transform: identity_transform.into(),
            color: [1.0, 1.0, 1.0],
            _padding: 0.0,
        };
        let road_identity_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Road Identity Instance Buffer"),
            contents: bytemuck::cast_slice(&[identity_instance]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            view_bind_group,
            view_buffer,
            car_vertex_buffer,
            road_vertex_buffer,
            road_vertex_count,
            car_instance_buffer,
            road_identity_instance_buffer,
            view_bind_group_layout,
            max_cars: max_cars as u32,
        })
    }
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    pub fn render(&mut self, state: &SimulationState, view_matrix: &Matrix4<f32>) -> Result<()> {
        // Update view uniforms
        let view_proj_array: [[f32; 4]; 4] = (*view_matrix).into();
        let uniforms = ViewUniforms {
            view_proj: view_proj_array,
        };
        self.queue.write_buffer(&self.view_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        // Update car instances
        let car_instances: Vec<CarInstance> = state.cars.iter().map(|car| {
            self.create_car_instance(car)
        }).collect();
        
        if !car_instances.is_empty() {
            self.queue.write_buffer(
                &self.car_instance_buffer,
                0,
                bytemuck::cast_slice(&car_instances),
            );
        }
        
        // Begin render pass
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.view_bind_group, &[]);
            
            // Render road
            render_pass.set_vertex_buffer(0, self.road_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.road_identity_instance_buffer.slice(..));
            render_pass.draw(0..self.road_vertex_count, 0..1);
            
            // Render cars
            if !state.cars.is_empty() {
                render_pass.set_vertex_buffer(0, self.car_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.car_instance_buffer.slice(..));
                render_pass.draw(0..6, 0..state.cars.len() as u32);
            }
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
    
    fn create_car_vertices() -> Vec<Vertex> {
        vec![
            // Triangle 1
            Vertex { position: [-0.5, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [0.5, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [0.0, 1.0, 0.0], color: [1.0, 0.0, 0.0] },
            // Triangle 2 (to make a simple car shape)
            Vertex { position: [-0.5, -1.0, 0.0], color: [0.8, 0.0, 0.0] },
            Vertex { position: [0.5, -1.0, 0.0], color: [0.8, 0.0, 0.0] },
            Vertex { position: [0.0, -0.5, 0.0], color: [0.8, 0.0, 0.0] },
        ]
    }
    
    fn create_road_vertices() -> Vec<Vertex> {
        // Create donut-shaped highway with lane markings
        let mut vertices = Vec::new();
        let segments = 64;
        let inner_radius = 150.0;
        let outer_radius = 200.0;
        let lane_width = 3.5;
        let lane_count = 3;
        
        // Create the road surface
        for i in 0..segments {
            let angle1 = (i as f32) * 2.0 * std::f32::consts::PI / (segments as f32);
            let angle2 = ((i + 1) as f32) * 2.0 * std::f32::consts::PI / (segments as f32);
            
            // Inner edge
            let inner1 = [inner_radius * angle1.cos(), inner_radius * angle1.sin(), 0.0];
            let inner2 = [inner_radius * angle2.cos(), inner_radius * angle2.sin(), 0.0];
            
            // Outer edge
            let outer1 = [outer_radius * angle1.cos(), outer_radius * angle1.sin(), 0.0];
            let outer2 = [outer_radius * angle2.cos(), outer_radius * angle2.sin(), 0.0];
            
            let road_color = [0.2, 0.2, 0.2]; // Dark gray for asphalt
            
            // Create two triangles for each segment
            vertices.push(Vertex { position: inner1, color: road_color });
            vertices.push(Vertex { position: outer1, color: road_color });
            vertices.push(Vertex { position: inner2, color: road_color });
            
            vertices.push(Vertex { position: inner2, color: road_color });
            vertices.push(Vertex { position: outer1, color: road_color });
            vertices.push(Vertex { position: outer2, color: road_color });
        }
        
        // Add lane markings (white dashed lines)
        let line_width = 0.15;
        let dash_length = 3.0; // meters
        let dash_spacing = 6.0; // meters
        let white_color = [0.9, 0.9, 0.9];
        
        for lane in 1..lane_count {
            let lane_radius = inner_radius + lane_width * lane as f32;
            let circumference = 2.0 * std::f32::consts::PI * lane_radius;
            let dash_cycle = dash_length + dash_spacing;
            let num_dashes = (circumference / dash_cycle) as usize;
            
            for dash in 0..num_dashes {
                let start_angle = (dash as f32) * dash_cycle / lane_radius;
                let end_angle = start_angle + (dash_length / lane_radius);
                
                let dash_segments = 8; // Segments per dash for smoothness
                for seg in 0..dash_segments {
                    let t1 = seg as f32 / dash_segments as f32;
                    let t2 = (seg + 1) as f32 / dash_segments as f32;
                    let a1 = start_angle + (end_angle - start_angle) * t1;
                    let a2 = start_angle + (end_angle - start_angle) * t2;
                    
                    let inner_r = lane_radius - line_width * 0.5;
                    let outer_r = lane_radius + line_width * 0.5;
                    
                    let p1 = [inner_r * a1.cos(), inner_r * a1.sin(), 0.01]; // Slightly above road
                    let p2 = [outer_r * a1.cos(), outer_r * a1.sin(), 0.01];
                    let p3 = [inner_r * a2.cos(), inner_r * a2.sin(), 0.01];
                    let p4 = [outer_r * a2.cos(), outer_r * a2.sin(), 0.01];
                    
                    // Two triangles for the dash segment
                    vertices.push(Vertex { position: p1, color: white_color });
                    vertices.push(Vertex { position: p2, color: white_color });
                    vertices.push(Vertex { position: p3, color: white_color });
                    
                    vertices.push(Vertex { position: p3, color: white_color });
                    vertices.push(Vertex { position: p2, color: white_color });
                    vertices.push(Vertex { position: p4, color: white_color });
                }
            }
        }
        
        vertices
    }
    
    fn create_car_instance(&self, car: &Car) -> CarInstance {
        // Create transformation matrix
        let scale = Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(car.length, car.width, 1.0));
        let rotation = Matrix4::from_euler_angles(0.0, 0.0, car.heading);
        let translation = Matrix4::new_translation(&nalgebra::Vector3::new(car.position.x, car.position.y, 0.0));
        
        let transform = translation * rotation * scale;
        let transform_array: [[f32; 4]; 4] = transform.into();
        
        // Color based on car type or speed
        let speed = car.velocity.magnitude();
        let max_speed = 30.0; // Assume max speed for color scaling
        let speed_ratio = (speed / max_speed).min(1.0);
        let color = [
            0.2 + speed_ratio * 0.8, // Red increases with speed
            0.8 - speed_ratio * 0.6, // Green decreases with speed  
            0.2,                      // Blue constant
        ];
        
        CarInstance {
            transform: transform_array,
            color,
            _padding: 0.0,
        }
    }
}