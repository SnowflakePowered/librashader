/*
librashader.h
SPDX-License-Identifier: MIT
This file is part of the librashader C headers.

Copyright 2022 chyyran

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

#ifndef __LIBRASHADER_H__
#define __LIBRASHADER_H__

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#if defined(_WIN32) && defined(LIBRA_RUNTIME_D3D11)
#include <d3d11.h>
#else
typedef void ID3D11Device;
typedef void ID3D11RenderTargetView;
typedef void ID3D11ShaderResourceView;
#endif
#if defined(LIBRA_RUNTIME_VULKAN)
#include <vulkan\vulkan.h>
#endif

/// Error codes for librashader error types.
enum LIBRA_ERRNO
#ifdef __cplusplus
    : int32_t
#endif    // __cplusplus
{
    LIBRA_ERRNO_UNKNOWN_ERROR = 0,
    LIBRA_ERRNO_INVALID_PARAMETER = 1,
    LIBRA_ERRNO_INVALID_STRING = 2,
    LIBRA_ERRNO_PRESET_ERROR = 3,
    LIBRA_ERRNO_PREPROCESS_ERROR = 4,
    LIBRA_ERRNO_SHADER_PARAMETER_ERROR = 5,
    LIBRA_ERRNO_REFLECT_ERROR = 6,
    LIBRA_ERRNO_RUNTIME_ERROR = 7,
};
#ifndef __cplusplus
typedef int32_t LIBRA_ERRNO;
#endif    // __cplusplus

/// A Direct3D 11 filter chain.
typedef struct _filter_chain_d3d11 _filter_chain_d3d11;

/// An OpenGL filter chain.
typedef struct _filter_chain_gl _filter_chain_gl;

/// A Vulkan filter chain.
typedef struct _filter_chain_vk _filter_chain_vk;

/// The error type for librashader.
typedef struct _libra_error _libra_error;

/// A shader preset including all specified parameters, textures, and paths to
/// specified shaders.
///
/// A shader preset can be used to create a filter chain runtime instance, or
/// reflected to get parameter metadata.
typedef struct _shader_preset _shader_preset;

/// A handle to a librashader error object.
typedef struct _libra_error *libra_error_t;

/// A handle to a shader preset object.
typedef struct _shader_preset *libra_shader_preset_t;

/// A preset parameter.
typedef struct libra_preset_param_t {
    /// The name of the parameter
    const char *name;
    /// The description of the parameter.
    const char *description;
    /// The initial value the parameter is set to.
    float initial;
    /// The minimum value that the parameter can be set to.
    float minimum;
    /// The maximum value that the parameter can be set to.
    float maximum;
    /// The step by which this parameter can be incremented or decremented.
    float step;
} libra_preset_param_t;

/// A list of preset parameters.
typedef struct libra_preset_param_list_t {
    /// A pointer to the parameter
    const struct libra_preset_param_t *parameters;
    /// The number of parameters in the list.
    uint64_t length;
    /// For internal use only.
    /// Changing this causes immediate undefined behaviour on freeing this
    /// parameter list.
    uint64_t _internal_alloc;
} libra_preset_param_list_t;

#if defined(LIBRA_RUNTIME_OPENGL)
/// A GL function loader that librashader needs to be initialized with.
typedef const void *(*libra_gl_loader_t)(const char *);
#endif

/// Options for filter chain creation.
typedef struct filter_chain_gl_opt_t {
    /// The GLSL version. Should be at least `330`.
    uint16_t gl_version;
    /// Whether or not to use the Direct State Access APIs. Only available on
    /// OpenGL 4.5+.
    bool use_dsa;
    /// Whether or not to explicitly disable mipmap generation regardless of
    /// shader preset settings.
    bool force_no_mipmaps;
} filter_chain_gl_opt_t;

#if defined(LIBRA_RUNTIME_OPENGL)
/// A handle to a OpenGL filter chain.
typedef struct _filter_chain_gl *libra_gl_filter_chain_t;
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// OpenGL parameters for the source image.
typedef struct libra_source_image_gl_t {
    /// A texture GLuint to the source image.
    uint32_t handle;
    /// The format of the source image.
    uint32_t format;
    /// The width of the source image.
    uint32_t width;
    /// The height of the source image.
    uint32_t height;
} libra_source_image_gl_t;
#endif

/// Defines the output viewport for a rendered frame.
typedef struct libra_viewport_t {
    /// The x offset in the viewport framebuffer to begin rendering from.
    float x;
    /// The y offset in the viewport framebuffer to begin rendering from.
    float y;
    /// The width of the viewport framebuffer.
    uint32_t width;
    /// The height of the viewport framebuffer.
    uint32_t height;
} libra_viewport_t;

#if defined(LIBRA_RUNTIME_OPENGL)
/// OpenGL parameters for the output framebuffer.
typedef struct libra_draw_framebuffer_gl_t {
    /// A framebuffer GLuint to the output framebuffer.
    uint32_t handle;
    /// A texture GLuint to the logical buffer of the output framebuffer.
    uint32_t texture;
    /// The format of the output framebuffer.
    uint32_t format;
} libra_draw_framebuffer_gl_t;
#endif

/// Options for each OpenGL shader frame.
typedef struct frame_gl_opt_t {
    /// Whether or not to clear the history buffers.
    bool clear_history;
    /// The direction of the frame. 1 should be vertical.
    int32_t frame_direction;
} frame_gl_opt_t;

/// Options for Direct3D11 filter chain creation.
typedef struct filter_chain_d3d11_opt_t {
    /// Use a deferred context to record shader rendering state.
    ///
    /// The deferred context will be executed on the immediate context
    /// with `RenderContextState = true`.
    bool use_deferred_context;
    /// Whether or not to explicitly disable mipmap
    /// generation regardless of shader preset settings.
    bool force_no_mipmaps;
} filter_chain_d3d11_opt_t;

#if defined(LIBRA_RUNTIME_D3D11)
/// A handle to a Direct3D11 filter chain.
typedef struct _filter_chain_d3d11 *libra_d3d11_filter_chain_t;
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// OpenGL parameters for the source image.
typedef struct libra_source_image_d3d11_t {
    /// A shader resource view into the source image
    const ID3D11ShaderResourceView *handle;
    /// The width of the source image.
    uint32_t width;
    /// The height of the source image.
    uint32_t height;
} libra_source_image_d3d11_t;
#endif

/// Options for each Direct3D11 shader frame.
typedef struct frame_d3d11_opt_t {
    /// Whether or not to clear the history buffers.
    bool clear_history;
    /// The direction of the frame. 1 should be vertical.
    int32_t frame_direction;
} frame_d3d11_opt_t;

#if defined(LIBRA_RUNTIME_VULKAN)
/// Handles required to instantiate vulkan
typedef struct libra_device_vk_t {
    /// A raw `VkPhysicalDevice` handle
    /// for the physical device that will perform rendering.
    VkPhysicalDevice physical_device;
    /// A raw `VkInstance` handle
    /// for the Vulkan instance that will perform rendering.
    VkInstance instance;
    /// A raw `VkDevice` handle
    /// for the device attached to the instance that will perform rendering.
    VkDevice device;
    /// The entry loader for the Vulkan library.
    PFN_vkGetInstanceProcAddr entry;
} libra_device_vk_t;
#endif

/// Options for filter chain creation.
typedef struct filter_chain_vk_opt_t {
    /// The number of frames in flight to keep. If zero, defaults to three.
    uint32_t frames_in_flight;
    /// Whether or not to explicitly disable mipmap generation regardless of
    /// shader preset settings.
    bool force_no_mipmaps;
} filter_chain_vk_opt_t;

#if defined(LIBRA_RUNTIME_VULKAN)
/// A handle to a Vulkan filter chain.
typedef struct _filter_chain_vk *libra_vk_filter_chain_t;
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Vulkan  parameters for the source image.
typedef struct libra_image_vk_t {
    /// A raw `VkImage` handle to the source image.
    VkImage handle;
    /// The `VkFormat` of the source image.
    VkFormat format;
    /// The width of the source image.
    uint32_t width;
    /// The height of the source image.
    uint32_t height;
} libra_image_vk_t;
#endif

/// Options for each Vulkan shader frame.
typedef struct frame_vk_opt_t {
    /// Whether or not to clear the history buffers.
    bool clear_history;
    /// The direction of the frame. 1 should be vertical.
    int32_t frame_direction;
} frame_vk_opt_t;

/// Function pointer definition for
/// libra_preset_create
typedef libra_error_t (*PFN_libra_preset_create)(const char *filename,
                                                 libra_shader_preset_t *out);

/// Function pointer definition for
/// libra_preset_free
typedef libra_error_t (*PFN_libra_preset_free)(libra_shader_preset_t *preset);

/// Function pointer definition for
/// libra_preset_set_param
typedef libra_error_t (*PFN_libra_preset_set_param)(
    libra_shader_preset_t *preset, const char *name, float value);

/// Function pointer definition for
/// libra_preset_get_param
typedef libra_error_t (*PFN_libra_preset_get_param)(
    libra_shader_preset_t *preset, const char *name, float *value);

/// Function pointer definition for
/// libra_preset_print
typedef libra_error_t (*PFN_libra_preset_print)(libra_shader_preset_t *preset);

/// Function pointer definition for
/// libra_preset_get_runtime_params
typedef libra_error_t (*PFN_libra_preset_get_runtime_params)(
    libra_shader_preset_t *preset, struct libra_preset_param_list_t *out);

/// Function pointer definition for
/// libra_preset_free_runtime_params
typedef libra_error_t (*PFN_libra_preset_free_runtime_params)(
    struct libra_preset_param_list_t preset);

/// Function pointer definition for libra_error_errno
typedef LIBRA_ERRNO (*PFN_libra_error_errno)(libra_error_t error);

/// Function pointer definition for libra_error_print
typedef int32_t (*PFN_libra_error_print)(libra_error_t error);

/// Function pointer definition for libra_error_free
typedef int32_t (*PFN_libra_error_free)(libra_error_t *error);

/// Function pointer definition for libra_error_write
typedef int32_t (*PFN_libra_error_write)(libra_error_t error, char **out);

/// Function pointer definition for libra_error_free_string
typedef int32_t (*PFN_libra_error_free_string)(char **out);

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_init_context
typedef libra_error_t (*PFN_libra_gl_init_context)(libra_gl_loader_t loader);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_create
typedef libra_error_t (*PFN_libra_gl_filter_chain_create)(
    libra_shader_preset_t *preset, const struct filter_chain_gl_opt_t *options,
    libra_gl_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_frame
typedef libra_error_t (*PFN_libra_gl_filter_chain_frame)(
    libra_gl_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_gl_t image, struct libra_viewport_t viewport,
    struct libra_draw_framebuffer_gl_t out, const float *mvp,
    const struct frame_gl_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_set_param
typedef libra_error_t (*PFN_libra_gl_filter_chain_set_param)(
    libra_gl_filter_chain_t *chain, const char *param_name, float value);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_get_param
typedef libra_error_t (*PFN_libra_gl_filter_chain_get_param)(
    libra_gl_filter_chain_t *chain, const char *param_name, float *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_set_active_pass_count
typedef libra_error_t (*PFN_libra_gl_filter_chain_set_active_pass_count)(
    libra_gl_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_get_active_pass_count
typedef libra_error_t (*PFN_libra_gl_filter_chain_get_active_pass_count)(
    libra_gl_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Function pointer definition for
/// libra_gl_filter_chain_free
typedef libra_error_t (*PFN_libra_gl_filter_chain_free)(
    libra_gl_filter_chain_t *chain);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_create
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_create)(
    libra_shader_preset_t *preset,
    const struct filter_chain_d3d11_opt_t *options, const ID3D11Device *device,
    libra_d3d11_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_frame
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_frame)(
    libra_d3d11_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_d3d11_t image, struct libra_viewport_t viewport,
    const ID3D11RenderTargetView *out, const float *mvp,
    const struct frame_d3d11_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_set_param
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_set_param)(
    libra_d3d11_filter_chain_t *chain, const char *param_name, float value);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_get_param
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_get_param)(
    libra_d3d11_filter_chain_t *chain, const char *param_name, float *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_set_active_pass_count
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_set_active_pass_count)(
    libra_d3d11_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_get_active_pass_count
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_get_active_pass_count)(
    libra_d3d11_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Function pointer definition for
/// libra_d3d11_filter_chain_free
typedef libra_error_t (*PFN_libra_d3d11_filter_chain_free)(
    libra_d3d11_filter_chain_t *chain);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_create
typedef libra_error_t (*PFN_libra_vk_filter_chain_create)(
    struct libra_device_vk_t vulkan, libra_shader_preset_t *preset,
    const struct filter_chain_vk_opt_t *options, libra_vk_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_frame
typedef libra_error_t (*PFN_libra_vk_filter_chain_frame)(
    libra_vk_filter_chain_t *chain, VkCommandBuffer command_buffer,
    size_t frame_count, struct libra_image_vk_t image,
    struct libra_viewport_t viewport, struct libra_image_vk_t out,
    const float *mvp, const struct frame_vk_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_set_param
typedef libra_error_t (*PFN_libra_vk_filter_chain_set_param)(
    libra_vk_filter_chain_t *chain, const char *param_name, float value);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_get_param
typedef libra_error_t (*PFN_libra_vk_filter_chain_get_param)(
    libra_vk_filter_chain_t *chain, const char *param_name, float *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_set_active_pass_count
typedef libra_error_t (*PFN_libra_vk_filter_chain_set_active_pass_count)(
    libra_vk_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_get_active_pass_count
typedef libra_error_t (*PFN_libra_vk_filter_chain_get_active_pass_count)(
    libra_vk_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Function pointer definition for
/// libra_vk_filter_chain_free
typedef libra_error_t (*PFN_libra_vk_filter_chain_free)(
    libra_vk_filter_chain_t *chain);
#endif

#ifdef __cplusplus
extern "C" {
#endif    // __cplusplus

/// Get the error code corresponding to this error object.
///
/// ## Safety
///   - `error` must be valid and initialized.
LIBRA_ERRNO libra_error_errno(libra_error_t error);

/// Print the error message.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise,
/// this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
int32_t libra_error_print(libra_error_t error);

/// Frees any internal state kept by the error.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise,
/// this function returns 0. The resulting error object becomes null.
/// ## Safety
///   - `error` must be null or a pointer to a valid and initialized instance of
///   `libra_error_t`.
int32_t libra_error_free(libra_error_t *error);

/// Writes the error message into `out`
///
/// If `error` is null, this function does nothing and returns 1. Otherwise,
/// this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
///   - `out` must be a non-null pointer. The resulting string must not be
///   modified.
int32_t libra_error_write(libra_error_t error, char **out);

/// Frees an error string previously allocated by `libra_error_write`.
///
/// After freeing, the pointer will be set to null.
/// ## Safety
///   - If `libra_error_write` is not null, it must point to a string previously
///   returned by `libra_error_write`.
///     Attempting to free anything else, including strings or objects from
///     other librashader functions, is immediate Undefined Behaviour.
int32_t libra_error_free_string(char **out);

/// Load a preset.
///
/// ## Safety
///  - `filename` must be either null or a valid, aligned pointer to a string
///  path to the shader preset.
///  - `out` must be either null, or an aligned pointer to an uninitialized or
///  invalid `libra_shader_preset_t`.
/// ## Returns
///  - If any parameters are null, `out` is unchanged, and this function returns
///  `LIBRA_ERR_INVALID_PARAMETER`.
libra_error_t libra_preset_create(const char *filename,
                                  libra_shader_preset_t *out);

/// Free the preset.
///
/// If `preset` is null, this function does nothing. The resulting value in
/// `preset` then becomes null.
///
/// ## Safety
/// - `preset` must be a valid and aligned pointer to a shader preset.
libra_error_t libra_preset_free(libra_shader_preset_t *preset);

/// Set the value of the parameter in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
libra_error_t libra_preset_set_param(libra_shader_preset_t *preset,
                                     const char *name, float value);

/// Get the value of the parameter as set in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
/// - `value` may be a pointer to a uninitialized `float`.
libra_error_t libra_preset_get_param(libra_shader_preset_t *preset,
                                     const char *name, float *value);

/// Pretty print the shader preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
libra_error_t libra_preset_print(libra_shader_preset_t *preset);

/// Get a list of runtime parameters.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `out` must be an aligned pointer to a `libra_preset_parameter_list_t`.
/// - The output struct should be treated as immutable. Mutating any struct
/// fields
///   in the returned struct may at best cause memory leaks, and at worse
///   cause undefined behaviour when later freed.
/// - It is safe to call `libra_preset_get_runtime_params` multiple times,
/// however
///   the output struct must only be freed once per call.
libra_error_t libra_preset_get_runtime_params(
    libra_shader_preset_t *preset, struct libra_preset_param_list_t *out);

/// Free the runtime parameters.
///
/// Unlike the other `free` functions provided by librashader,
/// `libra_preset_free_runtime_params` takes the struct directly.
/// The caller must take care to maintain the lifetime of any pointers
/// contained within the input `libra_preset_param_list_t`.
///
/// ## Safety
/// - Any pointers rooted at `parameters` becomes invalid after this function
/// returns,
///   including any strings accessible via the input
///   `libra_preset_param_list_t`. The caller must ensure that there are no live
///   pointers, aliased or unaliased, to data accessible via the input
///   `libra_preset_param_list_t`.
///
/// - Accessing any data pointed to via the input `libra_preset_param_list_t`
/// after it
///   has been freed is a use-after-free and is immediate undefined behaviour.
///
/// - If any struct fields of the input `libra_preset_param_list_t` was modified
/// from
///   their values given after `libra_preset_get_runtime_params`, this may
///   result in undefined behaviour.
libra_error_t libra_preset_free_runtime_params(
    struct libra_preset_param_list_t preset);

#if defined(LIBRA_RUNTIME_OPENGL)
/// Initialize the OpenGL Context for librashader.
///
/// This only has to be done once throughout the lifetime of the application,
/// unless for whatever reason you switch OpenGL loaders mid-flight.
///
/// ## Safety
/// Attempting to create a filter chain will fail if the GL context is not
/// initialized.
///
/// Reinitializing the OpenGL context with a different loader immediately
/// invalidates previous filter chain objects, and drawing with them causes
/// immediate undefined behaviour.
libra_error_t libra_gl_init_context(libra_gl_loader_t loader);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
libra_error_t libra_gl_filter_chain_create(
    libra_shader_preset_t *preset, const struct filter_chain_gl_opt_t *options,
    libra_gl_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Draw a frame with the given parameters for the given filter chain.
///
/// ## Safety
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or
/// invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16
/// consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a
/// valid `frame_gl_opt_t`
///    struct.
libra_error_t libra_gl_filter_chain_frame(
    libra_gl_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_gl_t image, struct libra_viewport_t viewport,
    struct libra_draw_framebuffer_gl_t out, const float *mvp,
    const struct frame_gl_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Sets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_gl_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_gl_filter_chain_set_param(libra_gl_filter_chain_t *chain,
                                              const char *param_name,
                                              float value);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Gets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_gl_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_gl_filter_chain_get_param(libra_gl_filter_chain_t *chain,
                                              const char *param_name,
                                              float *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Sets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_gl_filter_chain_t`.
libra_error_t libra_gl_filter_chain_set_active_pass_count(
    libra_gl_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Gets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_gl_filter_chain_t`.
libra_error_t libra_gl_filter_chain_get_active_pass_count(
    libra_gl_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_OPENGL)
/// Free a GL filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_gl_filter_chain_t`.
libra_error_t libra_gl_filter_chain_free(libra_gl_filter_chain_t *chain);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
libra_error_t libra_d3d11_filter_chain_create(
    libra_shader_preset_t *preset,
    const struct filter_chain_d3d11_opt_t *options, const ID3D11Device *device,
    libra_d3d11_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Draw a frame with the given parameters for the given filter chain.
///
/// ## Safety
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or
/// invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16
/// consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a
/// valid `frame_gl_opt_t`
///    struct.
libra_error_t libra_d3d11_filter_chain_frame(
    libra_d3d11_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_d3d11_t image, struct libra_viewport_t viewport,
    const ID3D11RenderTargetView *out, const float *mvp,
    const struct frame_d3d11_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Sets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_d3d11_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_d3d11_filter_chain_set_param(
    libra_d3d11_filter_chain_t *chain, const char *param_name, float value);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Gets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_d3d11_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_d3d11_filter_chain_get_param(
    libra_d3d11_filter_chain_t *chain, const char *param_name, float *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Sets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_d3d11_filter_chain_t`.
libra_error_t libra_d3d11_filter_chain_set_active_pass_count(
    libra_d3d11_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Gets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_d3d11_filter_chain_t`.
libra_error_t libra_d3d11_filter_chain_get_active_pass_count(
    libra_d3d11_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
/// Free a D3D11 filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_d3d11_filter_chain_t`.
libra_error_t libra_d3d11_filter_chain_free(libra_d3d11_filter_chain_t *chain);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - The handles provided in `vulkan` must be valid for the command buffers
/// that
///   `libra_vk_filter_chain_frame` will write to. Namely, the VkDevice must
///   have been
///    created with the `VK_KHR_dynamic_rendering` extension.
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
libra_error_t libra_vk_filter_chain_create(
    struct libra_device_vk_t vulkan, libra_shader_preset_t *preset,
    const struct filter_chain_vk_opt_t *options, libra_vk_filter_chain_t *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Records rendering commands for a frame with the given parameters for the
/// given filter chain to the input command buffer.
///
/// librashader will not do any queue submissions.
///
/// ## Safety
/// - `libra_vk_filter_chain_frame` **must not be called within a RenderPass**.
/// - `command_buffer` must be a valid handle to a `VkCommandBuffer` that is
/// ready for recording.
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or
/// invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16
/// consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a
/// valid `frame_vk_opt_t`
///    struct.
libra_error_t libra_vk_filter_chain_frame(
    libra_vk_filter_chain_t *chain, VkCommandBuffer command_buffer,
    size_t frame_count, struct libra_image_vk_t image,
    struct libra_viewport_t viewport, struct libra_image_vk_t out,
    const float *mvp, const struct frame_vk_opt_t *opt);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Sets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_vk_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_vk_filter_chain_set_param(libra_vk_filter_chain_t *chain,
                                              const char *param_name,
                                              float value);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Gets a parameter for the filter chain.
///
/// If the parameter does not exist, returns an error.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_vk_filter_chain_t`.
/// - `param_name` must be either null or a null terminated string.
libra_error_t libra_vk_filter_chain_get_param(libra_vk_filter_chain_t *chain,
                                              const char *param_name,
                                              float *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Sets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_vk_filter_chain_t`.
libra_error_t libra_vk_filter_chain_set_active_pass_count(
    libra_vk_filter_chain_t *chain, uint32_t value);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Gets the number of active passes for this chain.
///
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_vk_filter_chain_t`.
libra_error_t libra_vk_filter_chain_get_active_pass_count(
    libra_vk_filter_chain_t *chain, uint32_t *out);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
/// Free a Vulkan filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an
/// initialized `libra_vk_filter_chain_t`.
libra_error_t libra_vk_filter_chain_free(libra_vk_filter_chain_t *chain);
#endif

#ifdef __cplusplus
}    // extern "C"
#endif    // __cplusplus

#endif /* __LIBRASHADER_H__ */
