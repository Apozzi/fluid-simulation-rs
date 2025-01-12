#[macro_use]
extern crate glium;
mod support;

use glium::index::PrimitiveType;
use glium::{Display, Surface};
use glutin::surface::WindowSurface;
use support::{ApplicationContext, State};
use support::field::VectorField2D;
use support::mouse::Mouse;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

fn generate_color_matrix(grid_size: usize, time: f32) -> Vec<Vec<[f32; 3]>> {
    let mut color_matrix = vec![vec![[0.0, 0.0, 0.0]; grid_size]; grid_size];

    for row in 0..grid_size {
        for col in 0..grid_size {
            color_matrix[row][col] = [
                (row as f32 / grid_size as f32) + time.sin() * 0.1,  
                (col as f32 / grid_size as f32) + time.cos() * 0.1,  
                ((row + col) as f32 / (2 * grid_size) as f32) + time.sin() * 0.1 
            ];
        }
    }

    color_matrix
}

fn generate_grid_data(grid_size: usize, cell_size: f32, color_matrix: &Vec<Vec<[f32; 3]>>) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for row in 0..grid_size {
        for col in 0..grid_size {
            let x = -1.0 + col as f32 * cell_size;
            let y = 1.0 - row as f32 * cell_size;

            let cell_color = color_matrix[row][col];

            let v0 = vertices.len() as u32;
            vertices.push(Vertex {
                position: [x, y],
                color: cell_color,
            });
            vertices.push(Vertex {
                position: [x + cell_size, y],
                color: cell_color,
            });
            vertices.push(Vertex {
                position: [x + cell_size, y - cell_size],
                color: cell_color,
            });
            vertices.push(Vertex {
                position: [x, y - cell_size],
                color: cell_color,
            });

            indices.extend_from_slice(&[
                v0, v0 + 1, v0 + 2,
                v0, v0 + 2, v0 + 3,
            ]);
        }
    }

    (vertices, indices)
}

fn generate_arrows(_grid_size: usize, cell_size: f32, velocity_field: &VectorField2D) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for row in 0..velocity_field.height {
        for col in 0..velocity_field.width {
            let x = -1.0 + col as f32 * cell_size;
            let y = 1.0 - row as f32 * cell_size;

            let direction = velocity_field.field[row][col];
            let dx = direction[0] * cell_size;
            let dy = direction[1] * cell_size;

            let start = vertices.len() as u32;
            vertices.push(Vertex {
                position: [x + cell_size/2.0, y - cell_size/2.0],
                color: [1.0, 1.0, 1.0],
            });
            vertices.push(Vertex {
                position: [x + cell_size/2.0 + dx, y - cell_size/2.0 + dy],
                color: [1.0, 1.0, 1.0],
            });

            indices.push(start);
            indices.push(start + 1);

            let arrow_size = 0.2 * cell_size;
            let angle = dy.atan2(dx);
            let arrow_angle = 0.8;

            let left = [
                x + cell_size/2.0 + dx - arrow_size * (angle + arrow_angle).cos(),
                y - cell_size/2.0 + dy - arrow_size * (angle + arrow_angle).sin(),
            ];
            let right = [
                x + cell_size/2.0 + dx - arrow_size * (angle - arrow_angle).cos(),
                y - cell_size/2.0 + dy - arrow_size * (angle - arrow_angle).sin(),
            ];

            let left_idx = vertices.len() as u32;
            vertices.push(Vertex {
                position: left,
                color: [1.0, 1.0, 1.0],
            });
            vertices.push(Vertex {
                position: right,
                color: [1.0, 1.0, 1.0],
            });

            indices.push(start + 1);
            indices.push(left_idx);
            indices.push(start + 1);
            indices.push(left_idx + 1);
        }
    }

    (vertices, indices)
}

struct Application {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: glium::IndexBuffer<u32>,
    pub program: glium::Program,
    pub color_matrix: Vec<Vec<[f32; 3]>>,
    pub time: f32,
    pub velocity_field: VectorField2D,
    pub mouse_pressed: bool
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "Vector Field Example";

    fn new(display: &Display<WindowSurface>) -> Self {
        let field_width = 40;
        let field_height = 40;

        let velocity_field = VectorField2D {
            width: field_width,
            height: field_height,
            field: vec![vec![[0.5, 0.5]; field_width]; field_height],
        };

        let grid_size = 201;
        let cell_size = 0.01;

        let color_matrix = generate_color_matrix(grid_size, 0.0);
        let (vertices, indices) = generate_grid_data(grid_size, cell_size, &color_matrix);

        let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();
        let index_buffer = glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices).unwrap();

        let program = program!(display,
            100 => {
                vertex: "
                    #version 100
                    attribute lowp vec2 position;
                    attribute lowp vec3 color;
                    varying lowp vec3 vColor;
                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                        vColor = color;
                    }
                ",
                fragment: "
                    #version 100
                    varying lowp vec3 vColor;
                    void main() {
                        gl_FragColor = vec4(vColor, 1.0);
                    }
                ",
            },
        )
        .unwrap();

        Self {
            vertex_buffer,
            index_buffer,
            program,
            color_matrix,
            time: 0.0,
            velocity_field,
            mouse_pressed: false
        }
    }

    fn update(&mut self) {
        self.time += 0.01;
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        
        let grid_size = 201;
        let cell_size = 0.01;
        let color_matrix = generate_color_matrix(grid_size, self.time);
        let (vertices, indices) = generate_grid_data(grid_size, cell_size, &color_matrix);
        let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();
        let index_buffer = glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices).unwrap();

        frame.draw(
            &vertex_buffer,
            &index_buffer,
            &self.program,
            &uniform! {},
            &Default::default(),
        ).unwrap();

        let cell_size_field = 0.05;
        let (arrow_vertices, arrow_indices) = generate_arrows(
            self.velocity_field.width,
            cell_size_field,
            &self.velocity_field
        );
        
        if !arrow_vertices.is_empty() {
            let arrow_vertex_buffer = glium::VertexBuffer::new(display, &arrow_vertices).unwrap();
            let arrow_index_buffer = glium::IndexBuffer::new(
                display,
                PrimitiveType::LinesList,
                &arrow_indices
            ).unwrap();

            frame.draw(
                &arrow_vertex_buffer,
                &arrow_index_buffer,
                &self.program,
                &uniform! {},
                &Default::default(),
            ).unwrap();
        }

        frame.finish().unwrap();
    }

    fn handle_window_event(&mut self, event: &glium::winit::event::WindowEvent, _window: &glium::winit::window::Window) {
        match event {
            glium::winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *button == glium::winit::event::MouseButton::Left {
                    self.mouse_pressed = *state == glium::winit::event::ElementState::Pressed;
                    println!("Mouse pressed: {}", self.mouse_pressed);
                }
            },
            glium::winit::event::WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    let (delta_x, delta_y) = Mouse::get_delta();
                    let (x, y) = Mouse::get_position();
                    println!("Updating vector field: pos({}, {}), delta({}, {})", x, y, delta_x, delta_y);
                    self.update_vector_field(x, y, delta_x, delta_y);
                }
            },
            _ => {}
        }
    }
}

impl Application {
    fn update_vector_field(&mut self, x: i16, y: i16, delta_x: i16, delta_y: i16) {
        let magnitude = 0.1;
        
        let screen_ratio_x = self.velocity_field.width as f32 / 800.0;
        let screen_ratio_y = self.velocity_field.height as f32 / 600.0;
        
        let field_x = (x as f32 * screen_ratio_x).round() as usize;
        let field_y = (y as f32 * screen_ratio_y).round() as usize;
        
        for i in 0..self.velocity_field.height {
            for j in 0..self.velocity_field.width {
                let dx = j as f32 - field_x as f32;
                let dy = i as f32 - field_y as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                let influence = 1.0 / (1.0 + distance.powf(2.0));
                
                self.velocity_field.field[i][j][0] += delta_x as f32 * influence * magnitude;
                self.velocity_field.field[i][j][1] -= delta_y as f32 * influence * magnitude;
                
                let max_magnitude = 0.8;
                let current_magnitude = (self.velocity_field.field[i][j][0].powi(2) + 
                                       self.velocity_field.field[i][j][1].powi(2)).sqrt();
                
                if current_magnitude > max_magnitude {
                    self.velocity_field.field[i][j][0] *= max_magnitude / current_magnitude;
                    self.velocity_field.field[i][j][1] *= max_magnitude / current_magnitude;
                }
            }
        }
    }
}

fn main() {
    
    State::<Application>::run_loop();
}