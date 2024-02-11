use crate::error::Result;
use icrate::Metal::{
    MTLBlendFactorOneMinusSourceAlpha, MTLBlendFactorSourceAlpha, MTLDevice, MTLPixelFormat,
    MTLPrimitiveTopologyClassTriangle, MTLRenderPipelineColorAttachmentDescriptor,
    MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLVertexAttributeDescriptor,
    MTLVertexBufferLayoutDescriptor, MTLVertexDescriptor, MTLVertexFormatFloat2,
    MTLVertexStepFunctionPerVertex,
};
use librashader_reflect::reflect::ShaderReflection;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;

pub struct MetalGraphicsPipeline {
    format: MTLPixelFormat,
    pub layout: PipelineLayoutObjects,
    render_pipeline: Id<ProtocolObject<dyn MTLRenderPipelineState>>,
    device: Id<ProtocolObject<dyn MTLDevice>>,
}

pub struct PipelineLayoutObjects {}

impl PipelineLayoutObjects {
    unsafe fn create_vertex_descriptor() -> Id<MTLVertexDescriptor> {
        let descriptor = MTLVertexDescriptor::new();
        let attributes = descriptor.attributes();
        let layouts = descriptor.layouts();

        let binding = MTLVertexBufferLayoutDescriptor::new();

        let vertex_0 = MTLVertexAttributeDescriptor::new();
        let vertex_1 = MTLVertexAttributeDescriptor::new();

        // todo: RA uses a different vertex descriptor? We'll see if this works lol.
        // would be nice if it does but if not we'll just use the RA one...
        vertex_0.setFormat(MTLVertexFormatFloat2);

        // todo: this what RA does but like does it make sense for naga buffers..?
        // we don't want to use SPV-Cross if possible.
        vertex_0.setBufferIndex(4);
        vertex_0.setOffset(0);

        vertex_1.setFormat(MTLVertexFormatFloat2);
        vertex_1.setBufferIndex(4);
        vertex_1.setOffset(2 * std::mem::size_of::<f32>());

        attributes.setObject_atIndexedSubscript(Some(&vertex_0), 0);

        attributes.setObject_atIndexedSubscript(Some(&vertex_1), 1);

        binding.setStepFunction(MTLVertexStepFunctionPerVertex);
        binding.setStride(4 * std::mem::size_of::<f32>());
        layouts.setObject_atIndexedSubscript(Some(&binding), 0);

        descriptor
    }

    unsafe fn create_color_attachments(
        format: MTLPixelFormat,
    ) -> Id<MTLRenderPipelineColorAttachmentDescriptor> {
        let ca = MTLRenderPipelineColorAttachmentDescriptor::new();
        ca.setPixelFormat(format);
        ca.setBlendingEnabled(false);
        ca.setSourceAlphaBlendFactor(MTLBlendFactorSourceAlpha);
        ca.setSourceRGBBlendFactor(MTLBlendFactorSourceAlpha);
        ca.setDestinationAlphaBlendFactor(MTLBlendFactorOneMinusSourceAlpha);
        ca.setDetinationRGBBlendFactor(MTLBlendFactorOneMinusSourceAlpha);

        ca
    }

    pub fn create_pipeline(
        device: &ProtocolObject<dyn MTLDevice>,
        format: MTLPixelFormat,
    ) -> Result<Id<ProtocolObject<dyn MTLRenderPipelineState>>> {
        let descriptor = MTLRenderPipelineDescriptor::new();

        unsafe {
            let vertex = Self::create_vertex_descriptor();
            let ca = Self::create_color_attachments(format);

            descriptor.setInputPrimitiveTopology(MTLPrimitiveTopologyClassTriangle);
            descriptor.setVertexDescriptor(Some(&vertex));

            descriptor
                .colorAttachments()
                .setObject_atIndexedSubscript(Some(&ca), 0);

            descriptor.setRasterSampleCount(1);
        }

        Ok(device.newRenderPipelineStateWithDescriptor_error(descriptor.as_ref())?)
    }
}

impl MetalGraphicsPipeline {
    pub fn new(
        device: Id<ProtocolObject<dyn MTLDevice>>,
        reflection: &ShaderReflection,
        render_pass_format: MTLPixelFormat,
    ) -> Self {

        // device.newRenderPipelineStateWithDescriptor_error()
    }
}
