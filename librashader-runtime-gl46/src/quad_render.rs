use gl::types::{GLint, GLsizei, GLsizeiptr, GLuint};

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
            gl::CreateBuffers(1, &mut vbo);
            gl::NamedBufferData(
                vbo,
                std::mem::size_of_val(QUAD_VBO_DATA) as GLsizeiptr,
                QUAD_VBO_DATA.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
            gl::CreateVertexArrays(1, &mut vao);

            gl::EnableVertexArrayAttrib(vao, 0);
            gl::EnableVertexArrayAttrib(vao, 1);

            gl::VertexArrayVertexBuffer(vao, 0,
                                        vbo, 0, 4 * std::mem::size_of::<f32>() as GLint
            );

            gl::VertexArrayAttribFormat(vao, 0, 2,
                                        gl::FLOAT, gl::FALSE, 0);
            gl::VertexArrayAttribFormat(vao, 1, 2,
                                        gl::FLOAT, gl::FALSE,
                                        2 * std::mem::size_of::<f32>() as GLuint);

            gl::VertexArrayAttribBinding(vao, 0, 0);
            gl::VertexArrayAttribBinding(vao, 1, 0);

        }

        DrawQuad { vbo, vao }
    }

    pub fn bind_vao(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
        }
    }

    pub fn unbind_vao(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}
