use gl::types::{GLsizeiptr, GLuint};

#[rustfmt::skip]
static QUAD_VBO_DATA: &[f32; 16] = &[
    0.0f32, 0.0f32, 0.0f32, 0.0f32,
    1.0f32, 0.0f32, 1.0f32, 0.0f32,
    0.0f32, 1.0f32, 0.0f32, 1.0f32,
    1.0f32, 1.0f32, 1.0f32, 1.0f32,
];

pub struct DrawQuad {
    pub vbo: GLuint,
}

impl DrawQuad {
    pub fn new() -> DrawQuad {
        let mut vbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(QUAD_VBO_DATA) as GLsizeiptr,
                QUAD_VBO_DATA.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        DrawQuad { vbo }
    }
}
