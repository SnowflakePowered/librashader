use gl::types::{GLenum, GLuint};
use std::panic::catch_unwind;
use std::path::Path;

use crate::error::{FilterChainError, Result};
use crate::filter_chain::filter_impl::FilterChainImpl;
use crate::filter_chain::inner::FilterChainDispatch;
use crate::options::{FilterChainOptions, FrameOptions};
use crate::{Framebuffer, GLImage, Viewport};
use librashader_presets::ShaderPreset;

mod filter_impl;
mod inner;
mod parameters;

pub(crate) use filter_impl::FilterCommon;
use librashader_common::Size;

pub struct FilterChain {
    pub(in crate::filter_chain) filter: FilterChainDispatch,
}

impl FilterChain {
    pub fn load_from_preset(
        preset: ShaderPreset,
        options: Option<&FilterChainOptions>,
    ) -> Result<Self> {
        let result = catch_unwind(|| {
            if let Some(options) = options && options.use_dsa {
                return Ok(Self {
                    filter: FilterChainDispatch::DirectStateAccess(FilterChainImpl::load_from_preset(preset, Some(options))?)
                })
            }
            Ok(Self {
                filter: FilterChainDispatch::Compatibility(FilterChainImpl::load_from_preset(
                    preset, options,
                )?),
            })
        });
        match result {
            Err(_) => return Err(FilterChainError::GLLoadError),
            Ok(res) => res,
        }
    }

    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        path: impl AsRef<Path>,
        options: Option<&FilterChainOptions>,
    ) -> Result<Self> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(preset, options)
    }

    /// Process a frame with the input image.
    ///
    /// When this frame returns, `GL_FRAMEBUFFER` is bound to 0 if not using Direct State Access.
    /// Otherwise, it is untouched.
    pub fn frame(
        &mut self,
        input: &GLImage,
        viewport: &Viewport,
        frame_count: usize,
        options: Option<&FrameOptions>,
    ) -> Result<()> {
        match &mut self.filter {
            FilterChainDispatch::DirectStateAccess(p) => {
                p.frame(frame_count, viewport, input, options)
            }
            FilterChainDispatch::Compatibility(p) => p.frame(frame_count, viewport, input, options),
        }
    }
}
