#[macro_use]
extern crate glium;
mod support;

use glium::index::PrimitiveType;
use glium::{Display, Surface};
use glutin::surface::WindowSurface;
use support::{ApplicationContext, State};
use support::mouse::Mouse;
use support::WINDOW_HEIGHT;
use support::WINDOW_WIDTH;

#[derive(Debug, Clone)]
pub struct VectorField2D {
    pub width: usize,
    pub height: usize,
    pub field: Vec<Vec<[f32; 2]>>,
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

struct FluidField {
    density: Vec<Vec<f32>>,
    velocity: VectorField2D,
    width: usize,
    height: usize,
}

impl FluidField {
    fn new(width: usize, height: usize) -> Self {
        Self {
            density: vec![vec![0.0; width]; height],
            velocity: VectorField2D {
                width,
                height,
                field: vec![vec![[0.0, 0.0]; width]; height],
            },
            width,
            height,
        }
    }

    fn diffuse(&mut self, diff: f32, dt: f32) {
        // Básicamente é igualdade entre derivada parcial da densisade em relação (variação espacial da densidade) ou seja segunda derivada dos eixos (x,y)
        // multiplicada por um coeficiente (diff)
        let a = dt * diff * (self.width * self.height) as f32;
        let iterations = 20;

        let mut next_density = self.density.clone();

        for _ in 0..iterations {
            for i in 1..self.height-1 {
                for j in 1..self.width-1 {
                    next_density[i][j] = (self.density[i][j] + 
                        a * (next_density[i-1][j] + next_density[i+1][j] +
                             next_density[i][j-1] + next_density[i][j+1])) / (1.0 + 4.0 * a);
                }
            }
        }

        self.density = next_density;
    }

    // https://en.wikipedia.org/wiki/Advection
    fn advect(&mut self, dt: f32) {
        let mut next_density = vec![vec![0.0; self.width]; self.height];

        for i in 1..self.height-1 {
            for j in 1..self.width-1 {
                let velocity = self.velocity.field[i][j];
                
                let mut x = j as f32 - dt * velocity[0] * self.width as f32;
                let mut y = i as f32 - dt * velocity[1] * self.height as f32;
                
                x = x.max(0.5).min((self.width - 1) as f32 - 0.5);
                y = y.max(0.5).min((self.height - 1) as f32 - 0.5);
                
                let i0 = y.floor() as usize;
                let i1 = i0 + 1;
                let j0 = x.floor() as usize;
                let j1 = j0 + 1;
                
                let s1 = x - j0 as f32;
                let s0 = 1.0 - s1;
                let t1 = y - i0 as f32;
                let t0 = 1.0 - t1;
                
                // Faz a interpolação bilinear. (que é basicamente interpolação linear em duas dimensões)
                next_density[i][j] = 
                    t0 * (s0 * self.density[i0][j0] + s1 * self.density[i0][j1]) +
                    t1 * (s0 * self.density[i1][j0] + s1 * self.density[i1][j1]);
            }
        }

        self.density = next_density;
    }

    // Altera campo velocidade para que tenha conservação de massa {\displaystyle \nabla \cdot \mathbf {v} =0}:
    // E garantindo a lei da continuidade de Navier strokes.
    fn project(&mut self) {
        let mut div = vec![vec![0.0; self.width]; self.height];
        let mut p = vec![vec![0.0; self.width]; self.height];
        for i in 1..self.height-1 {
            for j in 1..self.width-1 {
                div[i][j] = -0.5 * (
                    self.velocity.field[i][j+1][0] - self.velocity.field[i][j-1][0] +
                    self.velocity.field[i+1][j][1] - self.velocity.field[i-1][j][1]
                );
            }
        }
        
        for _ in 0..20 {
            for i in 1..self.height-1 {
                for j in 1..self.width-1 {
                    p[i][j] = (div[i][j] + p[i-1][j] + p[i+1][j] + p[i][j-1] + p[i][j+1]) / 4.0;
                }
            }
        }
        for i in 1..self.height-1 {
            for j in 1..self.width-1 {
                self.velocity.field[i][j][0] -= 0.5 * (p[i][j+1] - p[i][j-1]);
                self.velocity.field[i][j][1] -= 0.5 * (p[i+1][j] - p[i-1][j]);
            }
        }
    }

    fn add_density(&mut self, x: usize, y: usize, amount: f32) {
        let radius = 5;
        for i in (y-radius).max(0)..(y+radius+1).min(self.height) {
            for j in (x-radius).max(0)..(x+radius+1).min(self.width) {
                let dx = j as f32 - x as f32;
                let dy = i as f32 - y as f32;
                let d = (dx * dx + dy * dy).sqrt();
                if d < radius as f32 {
                    self.density[i][j] += amount * (1.0 - d / radius as f32);
                }
            }
        }
    }
}

fn generate_color_from_density(fluid: &FluidField) -> Vec<Vec<[f32; 3]>> {
    let mut color_matrix = vec![vec![[0.0, 0.0, 0.0]; fluid.width]; fluid.height];

    for i in 0..fluid.height {
        for j in 0..fluid.width {
            let density = fluid.density[i][j].min(1.0);
            color_matrix[i][j] = [density, density * 0.7, density * 0.3];
        }
    }

    color_matrix
}

fn generate_arrows(_grid_size: usize, cell_size: f32, velocity_field: &VectorField2D) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Define o intervalo de amostragem (isso faz com que quantidade de flexas na tela diminua fazendo média das adjancentes)
    let sample_rate = 3;
    
    for row in (sample_rate..velocity_field.height-sample_rate).step_by(sample_rate) {
        for col in (sample_rate..velocity_field.width-sample_rate).step_by(sample_rate) {
            let x = -1.0 + col as f32 * cell_size * 0.4;
            let y = 1.0 - row as f32 * cell_size * 0.4;
            
            let mut avg_dx = 0.0;
            let mut avg_dy = 0.0;
            let mut count = 0.0;
            
            for i in -1..=1 {
                for j in -1..=1 {
                    let r = (row as i32 + i) as usize;
                    let c = (col as i32 + j) as usize;
                    if r < velocity_field.height && c < velocity_field.width {
                        avg_dx += velocity_field.field[r][c][0];
                        avg_dy += velocity_field.field[r][c][1];
                        count += 1.0;
                    }
                }
            }
            
            avg_dx /= count;
            avg_dy /= count;
            
            let magnitude = (avg_dx * avg_dx + avg_dy * avg_dy).sqrt();
            if magnitude < 0.01 {
                continue;
            }
            
            let scale = 2.0;
            let dx = avg_dx * cell_size * scale;
            let dy = -avg_dy * cell_size * scale;

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

            let arrow_size = 0.7 * cell_size * magnitude.min(1.0);
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

struct Application {
    pub vertex_buffer: glium::VertexBuffer<Vertex>,
    pub index_buffer: glium::IndexBuffer<u32>,
    pub program: glium::Program,
    pub fluid: FluidField,
    pub mouse_pressed: bool,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "Fluid Simulation";

    fn new(display: &Display<WindowSurface>) -> Self {
        let field_width = 100;
        let field_height = 100;
        let fluid = FluidField::new(field_width, field_height);

        let cell_size = 0.02;
        let color_matrix = generate_color_from_density(&fluid);
        let (vertices, indices) = generate_grid_data(field_width, cell_size, &color_matrix);

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
            fluid,
            mouse_pressed: false,
        }
    }

    fn update(&mut self) {
        let dt = 0.1;
        let diffusion = 0.0001;

        self.fluid.diffuse(diffusion, dt);
        self.fluid.project();
        self.fluid.advect(dt);
        self.fluid.project();
        
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        
        // Desenha o grid de densidade
        let cell_size = 0.02;
        let color_matrix = generate_color_from_density(&self.fluid);
        let (vertices, indices) = generate_grid_data(self.fluid.width, cell_size, &color_matrix);
        
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
            self.fluid.width,
            cell_size_field,
            &self.fluid.velocity
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
                }
            },
            glium::winit::event::WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    let (delta_x, delta_y) = Mouse::get_delta();
                    let (x, y) = Mouse::get_position();
                    self.update_vector_field(x, y, delta_x, delta_y);
                }
            },
            _ => {}
        }
    }
}

impl Application {
    fn update_vector_field(&mut self, x: i16, y: i16, delta_x: i16, delta_y: i16) {
        let magnitude = 0.2;
        
        let screen_ratio_x = self.fluid.width as f32 / WINDOW_WIDTH as f32;
        let screen_ratio_y = self.fluid.height as f32 / WINDOW_HEIGHT as f32;
        
        let field_x = (x as f32 * screen_ratio_x).round() as usize;
        let field_y = (y as f32 * screen_ratio_y).round() as usize;

        self.fluid.add_density(field_x, field_y, 2.0);

        
        for i in 0..self.fluid.height {
            for j in 0..self.fluid.width {
                let dx = j as f32 - field_x as f32;
                let dy = i as f32 - field_y as f32;
                let distance = (dx * dx + dy * dy).sqrt();
                let influence = 1.0 / (1.0 + distance.powf(2.0));
                
                self.fluid.velocity.field[i][j][0] += delta_x as f32 * influence * magnitude;
                self.fluid.velocity.field[i][j][1] += delta_y as f32 * influence * magnitude;
                
                let max_magnitude = 0.8;
                let current_magnitude = (self.fluid.velocity.field[i][j][0].powi(2) + 
                                       self.fluid.velocity.field[i][j][1].powi(2)).sqrt();
                
                if current_magnitude > max_magnitude {
                    self.fluid.velocity.field[i][j][0] *= max_magnitude / current_magnitude;
                    self.fluid.velocity.field[i][j][1] *= max_magnitude / current_magnitude;
                }
            }
        }
    }
}

fn main() {
    
    State::<Application>::run_loop();
}