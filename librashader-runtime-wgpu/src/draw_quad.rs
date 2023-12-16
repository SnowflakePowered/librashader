use array_concat::concat_arrays;
use librashader_runtime::quad::QuadType;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferDescriptor, Device, RenderPass};

#[rustfmt::skip]
const VBO_OFFSCREEN: [f32; 16] = [
    // Offscreen
    -1.0f32, -1.0, 0.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
    1.0, -1.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

#[rustfmt::skip]
const VBO_DEFAULT_FINAL: [f32; 16] = [
    // Final
    0.0f32, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 1.0,
    1.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

static VBO_DATA: &[f32; 32] = &concat_arrays!(VBO_OFFSCREEN, VBO_DEFAULT_FINAL);
pub struct DrawQuad {
    buffer: Buffer,
}

impl DrawQuad {
    pub fn new(device: &Device) -> DrawQuad {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("librashader vbo"),
            contents: bytemuck::cast_slice(VBO_DATA),
            usage: wgpu::BufferUsages::VERTEX,
        });

        DrawQuad { buffer }
    }

    pub fn bind_vbo_for_frame<'a, 'b: 'a>(&'b self, cmd: &mut RenderPass<'a>) {
        cmd.set_vertex_buffer(0, self.buffer.slice(0..));
    }

    pub fn draw_quad<'a, 'b: 'a>(&'b self, cmd: &mut RenderPass<'a>, vbo: QuadType) {
        let offset = match vbo {
            QuadType::Offscreen => 0..3,
            QuadType::Final => 1..4,
        };

        cmd.draw(offset, 0..1)
    }
}
