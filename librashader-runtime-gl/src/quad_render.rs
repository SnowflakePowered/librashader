use gl::types::{GLsizei, GLsizeiptr, GLuint};

#[rustfmt::skip]
static QUAD_VBO_DATA: &[f32; 16] = &[
    0.0f32, 0.0f32, 0.0f32, 0.0f32,
    1.0f32, 0.0f32, 1.0f32, 0.0f32,
    0.0f32, 1.0f32, 0.0f32, 1.0f32,
    1.0f32, 1.0f32, 1.0f32, 1.0f32,
];

pub struct DrawQuad {
    vbo: GLuint,
    vao: GLuint,
}

impl DrawQuad {
    pub fn new() -> DrawQuad {
        let mut vbo = 0;
        let mut vao = 0;

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
            gl::GenVertexArrays(1, &mut vao);
        }


        DrawQuad { vbo, vao }
    }

    pub fn bind_vao(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            // the provided pointers are of OpenGL provenance with respect to the buffer bound to quad_vbo,
            // and not a known provenance to the Rust abstract machine, therefore we give it invalid pointers.
            // that are inexpressible in Rust
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLsizei,
                std::ptr::invalid(0),
            );
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLsizei,
                std::ptr::invalid(2 * std::mem::size_of::<f32>()),
            );
        }
    }

    pub fn unbind_vao(&self) {
        unsafe {
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::DisableVertexAttribArray(0);
            gl::DisableVertexAttribArray(1);
        }
    }
}
