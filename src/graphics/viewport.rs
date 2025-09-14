use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};
use nalgebra::{Matrix4, Vector3, Point3};

pub struct Viewport {
    // Camera properties
    pub position: Vector3<f32>,
    pub zoom: f32,
    pub target: Vector3<f32>,
    
    // Input state
    is_dragging: bool,
    last_mouse_pos: (f32, f32),
    mouse_pos: (f32, f32),
    
    // Viewport dimensions
    width: f32,
    height: f32,
    
    // Animation
    target_position: Vector3<f32>,
    target_zoom: f32,
    animation_speed: f32,
    
    // Controls
    pan_speed: f32,
    zoom_speed: f32,
    min_zoom: f32,
    max_zoom: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            zoom: 1.0,
            target: Vector3::new(0.0, 0.0, 0.0),
            is_dragging: false,
            last_mouse_pos: (0.0, 0.0),
            mouse_pos: (0.0, 0.0),
            width,
            height,
            target_position: Vector3::new(0.0, 0.0, 0.0),
            target_zoom: 1.0,
            animation_speed: 8.0,
            pan_speed: 1.0,
            zoom_speed: 0.1,
            min_zoom: 0.1,
            max_zoom: 10.0,
        }
    }
    
    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }
    
    pub fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        match button {
            MouseButton::Left => {
                match state {
                    ElementState::Pressed => {
                        self.is_dragging = true;
                        self.last_mouse_pos = self.mouse_pos;
                    }
                    ElementState::Released => {
                        self.is_dragging = false;
                    }
                }
            }
            _ => {}
        }
    }
    
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) {
        self.last_mouse_pos = self.mouse_pos;
        self.mouse_pos = (x, y);
        
        if self.is_dragging {
            let delta_x = (x - self.last_mouse_pos.0) / self.zoom;
            let delta_y = (y - self.last_mouse_pos.1) / self.zoom;
            
            // Convert screen coordinates to world coordinates
            self.target_position.x -= delta_x * self.pan_speed;
            self.target_position.y += delta_y * self.pan_speed; // Flip Y axis
        }
    }
    
    pub fn handle_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        let zoom_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => *y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
        };
        
        let zoom_factor = 1.0 + zoom_delta * self.zoom_speed;
        self.target_zoom = (self.target_zoom * zoom_factor).clamp(self.min_zoom, self.max_zoom);
        
        // Zoom towards mouse cursor
        let mouse_world = self.screen_to_world(self.mouse_pos.0, self.mouse_pos.1);
        let zoom_direction = mouse_world - self.target_position;
        self.target_position += zoom_direction * (1.0 - 1.0 / zoom_factor) * 0.1;
    }
    
    pub fn handle_keyboard_input(&mut self, input: &winit::event::KeyEvent) {
        if input.state == ElementState::Pressed {
            let movement_speed = 50.0 / self.zoom;
            
            if let PhysicalKey::Code(keycode) = input.physical_key {
                match keycode {
                    KeyCode::ArrowUp | KeyCode::KeyW => {
                        self.target_position.y += movement_speed;
                    }
                    KeyCode::ArrowDown | KeyCode::KeyS => {
                        self.target_position.y -= movement_speed;
                    }
                    KeyCode::ArrowLeft | KeyCode::KeyA => {
                        self.target_position.x -= movement_speed;
                    }
                    KeyCode::ArrowRight | KeyCode::KeyD => {
                        self.target_position.x += movement_speed;
                    }
                    KeyCode::Home => {
                        // Reset view to origin
                        self.target_position = Vector3::new(0.0, 0.0, 0.0);
                        self.target_zoom = 1.0;
                    }
                    KeyCode::Equal | KeyCode::NumpadAdd => {
                        self.target_zoom = (self.target_zoom * 1.2).min(self.max_zoom);
                    }
                    KeyCode::Minus | KeyCode::NumpadSubtract => {
                        self.target_zoom = (self.target_zoom / 1.2).max(self.min_zoom);
                    }
                    _ => {}
                }
            }
        }
    }
    
    pub fn update(&mut self) {
        let dt = 1.0 / 60.0; // Assume 60 FPS for smooth animation
        let interpolation_factor = 1.0 - (-self.animation_speed * dt).exp();
        
        // Smoothly interpolate to target position and zoom
        self.position += (self.target_position - self.position) * interpolation_factor;
        self.zoom += (self.target_zoom - self.zoom) * interpolation_factor;
    }
    
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        // Create orthographic projection matrix
        let aspect_ratio = self.width / self.height;
        let view_width = 400.0 / self.zoom; // Base view width
        let view_height = view_width / aspect_ratio;
        
        let left = self.position.x - view_width / 2.0;
        let right = self.position.x + view_width / 2.0;
        let bottom = self.position.y - view_height / 2.0;
        let top = self.position.y + view_height / 2.0;
        let near = -100.0;
        let far = 100.0;
        
        // Create orthographic projection matrix
        Matrix4::new_orthographic(left, right, bottom, top, near, far)
    }
    
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Vector3<f32> {
        let aspect_ratio = self.width / self.height;
        let view_width = 400.0 / self.zoom;
        let view_height = view_width / aspect_ratio;
        
        // Convert from screen coordinates [0, width] x [0, height] 
        // to normalized coordinates [-1, 1] x [-1, 1]
        let norm_x = (2.0 * screen_x / self.width) - 1.0;
        let norm_y = 1.0 - (2.0 * screen_y / self.height); // Flip Y
        
        // Convert to world coordinates
        let world_x = self.position.x + norm_x * view_width / 2.0;
        let world_y = self.position.y + norm_y * view_height / 2.0;
        
        Vector3::new(world_x, world_y, 0.0)
    }
    
    pub fn world_to_screen(&self, world_pos: &Vector3<f32>) -> (f32, f32) {
        let aspect_ratio = self.width / self.height;
        let view_width = 400.0 / self.zoom;
        let view_height = view_width / aspect_ratio;
        
        // Convert world coordinates to normalized coordinates
        let norm_x = (world_pos.x - self.position.x) / (view_width / 2.0);
        let norm_y = (world_pos.y - self.position.y) / (view_height / 2.0);
        
        // Convert to screen coordinates
        let screen_x = (norm_x + 1.0) * self.width / 2.0;
        let screen_y = (1.0 - norm_y) * self.height / 2.0; // Flip Y
        
        (screen_x, screen_y)
    }
    
    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }
    
    pub fn get_position(&self) -> &Vector3<f32> {
        &self.position
    }
    
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
        self.target_position = position;
    }
    
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
        self.target_zoom = self.zoom;
    }
}