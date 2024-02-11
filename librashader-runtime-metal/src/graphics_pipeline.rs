use crate::error::{FilterChainError, Result};
use icrate::Foundation::NSString;
use icrate::Metal::{
    MTLBlendFactorOneMinusSourceAlpha, MTLBlendFactorSourceAlpha, MTLCommandBuffer,
    MTLCommandEncoder, MTLDevice, MTLFunction, MTLLibrary, MTLLoadActionDontCare, MTLPixelFormat,
    MTLPrimitiveTopologyClassTriangle, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLRenderPipelineColorAttachmentDescriptor, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState, MTLScissorRect, MTLStoreActionStore, MTLTexture,
    MTLVertexAttributeDescriptor, MTLVertexBufferLayoutDescriptor, MTLVertexDescriptor,
    MTLVertexFormatFloat2, MTLVertexStepFunctionPerVertex, MTLViewport,
};
use librashader_reflect::back::msl::{CrossMslContext, NagaMslContext};
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::render_target::RenderTarget;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;

pub struct MetalGraphicsPipeline {
    pub layout: PipelineLayoutObjects,
    render_pipeline: Id<ProtocolObject<dyn MTLRenderPipelineState>>,
}

pub struct PipelineLayoutObjects {
    vertex_lib: Id<ProtocolObject<dyn MTLLibrary>>,
    fragment_lib: Id<ProtocolObject<dyn MTLLibrary>>,
    vertex_entry: Id<ProtocolObject<dyn MTLFunction>>,
    fragment_entry: Id<ProtocolObject<dyn MTLFunction>>,
    device: Id<ProtocolObject<dyn MTLDevice>>,
}

trait MslEntryPoint {
    fn entry_point() -> Id<NSString>;
}

impl MslEntryPoint for CrossMslContext {
    fn entry_point() -> Id<NSString> {
        NSString::from_str("main0")
    }
}

impl MslEntryPoint for NagaMslContext {
    fn entry_point() -> Id<NSString> {
        NSString::from_str("main_")
    }
}

impl PipelineLayoutObjects {
    pub fn new<T: MslEntryPoint>(
        shader_assembly: &ShaderCompilerOutput<String, T>,
        device: Id<ProtocolObject<dyn MTLDevice>>,
    ) -> Result<Self> {
        let entry = T::entry_point();

        let vertex = NSString::from_str(&shader_assembly.vertex);
        let vertex = device.newLibraryWithSource_options_error(&vertex, None)?;
        let vertex_entry = vertex
            .newFunctionWithName(&entry)
            .ok_or(FilterChainError::ShaderWrongEntryName)?;

        let fragment = NSString::from_str(&shader_assembly.fragment);
        let fragment = device.newLibraryWithSource_options_error(&fragment, None)?;
        let fragment_entry = fragment
            .newFunctionWithName(&entry)
            .ok_or(FilterChainError::ShaderWrongEntryName)?;

        Ok(Self {
            vertex_lib: vertex,
            fragment_lib: fragment,
            vertex_entry,
            fragment_entry,
            device,
        })
    }

    unsafe fn create_vertex_descriptor() -> Id<MTLVertexDescriptor> {
        let descriptor = MTLVertexDescriptor::new();
        let attributes = descriptor.attributes();
        let layouts = descriptor.layouts();

        let binding = MTLVertexBufferLayoutDescriptor::new();

        let vertex_0 = MTLVertexAttributeDescriptor::new();
        let vertex_1 = MTLVertexAttributeDescriptor::new();

        // hopefully metal fills in vertices otherwise we'll need to use the vec4 stuff.
        vertex_0.setFormat(MTLVertexFormatFloat2);
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
        ca.setDestinationRGBBlendFactor(MTLBlendFactorOneMinusSourceAlpha);

        ca
    }

    pub fn create_pipeline(
        &self,
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

            descriptor.setVertexFunction(Some(&self.vertex_entry));
            descriptor.setFragmentFunction(Some(&self.fragment_entry));
        }

        Ok(self
            .device
            .newRenderPipelineStateWithDescriptor_error(descriptor.as_ref())?)
    }
}

impl MetalGraphicsPipeline {
    pub fn new<T: MslEntryPoint>(
        device: Id<ProtocolObject<dyn MTLDevice>>,
        shader_assembly: &ShaderCompilerOutput<String, T>,
        render_pass_format: MTLPixelFormat,
    ) -> Result<Self> {
        let layout = PipelineLayoutObjects::new(shader_assembly, device)?;
        let pipeline = layout.create_pipeline(render_pass_format)?;
        Ok(Self {
            layout,
            render_pipeline: pipeline,
        })
    }

    pub fn recompile(&mut self, format: MTLPixelFormat) -> Result<()> {
        let render_pipeline = self.layout.create_pipeline(format)?;
        self.render_pipeline = render_pipeline;
        Ok(())
    }

    pub fn begin_rendering<'pass>(
        &self,
        output: RenderTarget<&'pass ProtocolObject<dyn MTLTexture>>,
        buffer: Id<ProtocolObject<dyn MTLCommandBuffer>>,
    ) -> Result<Id<ProtocolObject<dyn MTLRenderCommandEncoder>>> {
        unsafe {
            let descriptor = MTLRenderPassDescriptor::new();
            let ca = descriptor.colorAttachments().objectAtIndexedSubscript(0);
            ca.setLoadAction(MTLLoadActionDontCare);
            ca.setStoreAction(MTLStoreActionStore);
            ca.setTexture(Some(output.output));

            let rpass = buffer
                .renderCommandEncoderWithDescriptor(&descriptor)
                .ok_or(FilterChainError::FailedToCreateRenderPass)?;

            rpass.setScissorRect(MTLScissorRect {
                x: output.x as usize,
                y: output.y as usize,
                width: output.output.width(),
                height: output.output.height(),
            });

            rpass.setViewport(MTLViewport {
                originX: output.x as f64,
                originY: output.y as f64,
                width: output.output.width() as f64,
                height: output.output.height() as f64,
                znear: 0.0,
                zfar: 1.0,
            });

            rpass.setRenderPipelineState(&self.render_pipeline);

            Ok(rpass)
        }
    }
}
