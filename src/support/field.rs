#[derive(Debug, Clone)]
pub struct VectorField2D {
    pub width: usize,
    pub height: usize,
    pub field: Vec<Vec<[f32; 2]>>,
}

#[derive(Debug, Clone)]
pub struct ColorField2D {
    pub width: usize,
    pub height: usize,
    pub field: Vec<Vec<f32>>,
}

