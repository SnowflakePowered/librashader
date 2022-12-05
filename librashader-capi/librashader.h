#ifndef __LIBRASHADER_H__
#define __LIBRASHADER_H__

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#ifdef _WIN32
#include <d3d11.h>
#else
typedef void ID3D11Device;typedef void ID3D11RenderTargetView;typedef void ID3D1ShaderResourceView;
#endif

/// Error codes for librashader error types.
enum LIBRA_ERRNO
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
  LIBRA_ERRNO_UNKNOWN_ERROR = 0,
  LIBRA_ERRNO_INVALID_PARAMETER = 1,
  LIBRA_ERRNO_INVALID_PATH = 2,
  LIBRA_ERRNO_PRESET_ERROR = 3,
  LIBRA_ERRNO_PREPROCESS_ERROR = 4,
  LIBRA_ERRNO_RUNTIME_ERROR = 5,
};
#ifndef __cplusplus
typedef int32_t LIBRA_ERRNO;
#endif // __cplusplus

typedef struct _filter_chain_d3d11 _filter_chain_d3d11;

typedef struct _filter_chain_gl _filter_chain_gl;

/// The error type for librashader.
typedef struct _libra_error _libra_error;

/// A shader preset including all specified parameters, textures, and paths to specified shaders.
///
/// A shader preset can be used to create a filter chain runtime instance, or reflected to get
/// parameter metadata.
typedef struct _shader_preset _shader_preset;

typedef struct _libra_error *libra_error_t;

typedef struct _shader_preset *libra_shader_preset_t;

/// A GL function loader that librashader needs to be initialized with.
typedef const void *(*gl_loader_t)(const char*);

/// Options for filter chain creation.
typedef struct filter_chain_gl_opt_t {
  /// The GLSL version. Should be at least `330`.
  uint16_t gl_version;
  /// Whether or not to use the Direct State Access APIs. Only available on OpenGL 4.5+.
  bool use_dsa;
} filter_chain_gl_opt_t;

typedef struct _filter_chain_gl *libra_gl_filter_chain_t;

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

/// Parameters for the output viewport.
typedef struct libra_viewport_t {
  float x;
  float y;
  uint32_t width;
  uint32_t height;
} libra_viewport_t;

/// OpenGL parameters for the output framebuffer.
typedef struct libra_draw_framebuffer_gl_t {
  /// A framebuffer GLuint to the output framebuffer.
  uint32_t handle;
  /// A texture GLuint to the logical buffer of the output framebuffer.
  uint32_t texture;
  /// The format of the output framebuffer.
  uint32_t format;
} libra_draw_framebuffer_gl_t;

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
} filter_chain_d3d11_opt_t;

typedef struct _filter_chain_d3d11 *libra_d3d11_filter_chain_t;

/// OpenGL parameters for the source image.
typedef struct libra_source_image_d3d11_t {
  /// A shader resource view into the source image
  const ID3D11ShaderResourceView *handle;
  /// The width of the source image.
  uint32_t width;
  /// The height of the source image.
  uint32_t height;
} libra_source_image_d3d11_t;

/// Options for each Direct3D11 shader frame.
typedef struct frame_d3d11_opt_t {
  /// Whether or not to clear the history buffers.
  bool clear_history;
  /// The direction of the frame. 1 should be vertical.
  int32_t frame_direction;
} frame_d3d11_opt_t;

typedef libra_error_t (*PFN_lbr_preset_free)(libra_shader_preset_t *preset);

typedef libra_error_t (*PFN_lbr_preset_set_param)(libra_shader_preset_t *preset, const char *name, float value);

typedef libra_error_t (*PFN_lbr_preset_get_param)(libra_shader_preset_t *preset, const char *name, float *value);

typedef libra_error_t (*PFN_lbr_preset_print)(libra_shader_preset_t *preset);

typedef libra_error_t (*PFN_lbr_preset_get_runtime_param_names)(libra_shader_preset_t *preset, const char **value);

typedef LIBRA_ERRNO (*PFN_lbr_error_errno)(libra_error_t error);

typedef int32_t (*PFN_lbr_error_print)(libra_error_t error);

typedef int32_t (*PFN_lbr_error_free)(libra_error_t *error);

typedef int32_t (*PFN_lbr_error_write)(libra_error_t error, char **out);

typedef int32_t (*PFN_lbr_error_free_string)(char **out);

typedef libra_error_t (*PFN_lbr_gl_init_context)(gl_loader_t loader);

typedef libra_error_t (*PFN_lbr_gl_filter_chain_create)(libra_shader_preset_t *preset, const struct filter_chain_gl_opt_t *options, libra_gl_filter_chain_t *out);

typedef libra_error_t (*PFN_lbr_gl_filter_chain_frame)(libra_gl_filter_chain_t *chain, size_t frame_count, struct libra_source_image_gl_t image, struct libra_viewport_t viewport, struct libra_draw_framebuffer_gl_t out, const float *mvp, const struct frame_gl_opt_t *opt);

typedef libra_error_t (*PFN_lbr_gl_filter_chain_free)(libra_gl_filter_chain_t *chain);

typedef libra_error_t (*PFN_lbr_d3d11_filter_chain_create)(libra_shader_preset_t *preset, const struct filter_chain_d3d11_opt_t *options, const ID3D11Device *device, libra_d3d11_filter_chain_t *out);

typedef libra_error_t (*PFN_lbr_d3d11_filter_chain_frame)(libra_d3d11_filter_chain_t *chain, size_t frame_count, struct libra_source_image_d3d11_t image, struct libra_viewport_t viewport, const ID3D11RenderTargetView *out, const float *mvp, const struct frame_d3d11_opt_t *opt);

typedef libra_error_t (*PFN_lbr_d3d11_filter_chain_free)(libra_d3d11_filter_chain_t *chain);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/// Get the error code corresponding to this error object.
///
/// ## Safety
///   - `error` must be valid and initialized.
LIBRA_ERRNO libra_error_errno(libra_error_t error);

/// Print the error message.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
int32_t libra_error_print(libra_error_t error);

/// Frees any internal state kept by the error.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// The resulting error object becomes null.
/// ## Safety
///   - `error` must be null or a pointer to a valid and initialized instance of `libra_error_t`.
int32_t libra_error_free(libra_error_t *error);

/// Writes the error message into `out`
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
///   - `out` must be a non-null pointer. The resulting string must not be modified.
int32_t libra_error_write(libra_error_t error,
                          char **out);

/// Frees an error string previously allocated by `libra_error_write`.
///
/// After freeing, the pointer will be set to null.
/// ## Safety
///   - If `libra_error_write` is not null, it must point to a string previously returned by `libra_error_write`.
///     Attempting to free anything else, including strings or objects from other librashader functions, is immediate
///     Undefined Behaviour.
int32_t libra_error_free_string(char **out);

/// Load a preset.
///
/// ## Safety
///  - `filename` must be either null or a valid, aligned pointer to a string path to the shader preset.
///  - `out` must be either null, or an aligned pointer to an uninitialized or invalid `libra_shader_preset_t`.
/// ## Returns
///  - If any parameters are null, `out` is unchanged, and this function returns `LIBRA_ERR_INVALID_PARAMETER`.
libra_error_t libra_preset_create(const char *filename,
                                  libra_shader_preset_t *out);

/// Free the preset.
///
/// If `preset` is null, this function does nothing. The resulting value in `preset` then becomes
/// null.
///
/// ## Safety
/// - `preset` must be a valid and aligned pointer to a shader preset.
libra_error_t libra_preset_free(libra_shader_preset_t *preset);

/// Set the value of the parameter in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
libra_error_t libra_preset_set_param(libra_shader_preset_t *preset, const char *name, float value);

/// Get the value of the parameter as set in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
/// - `value` may be a pointer to a uninitialized `float`.
libra_error_t libra_preset_get_param(libra_shader_preset_t *preset, const char *name, float *value);

/// Pretty print the shader preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
libra_error_t libra_preset_print(libra_shader_preset_t *preset);

/// Get a list of runtime parameter names.
///
/// The returned value can not currently be freed.
/// This function should be considered in progress. Its use is discouraged.
libra_error_t libra_preset_get_runtime_param_names(libra_shader_preset_t *preset,
                                                   const char **value);

/// Initialize the OpenGL Context for librashader.
///
/// ## Safety
/// Attempting to create a filter chain will fail.
///
/// Reinitializing the OpenGL context with a different loader immediately invalidates previous filter
/// chain objects, and drawing with them causes immediate undefined behaviour.
libra_error_t libra_gl_init_context(gl_loader_t loader);

/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
libra_error_t libra_gl_filter_chain_create(libra_shader_preset_t *preset,
                                           const struct filter_chain_gl_opt_t *options,
                                           libra_gl_filter_chain_t *out);

/// Draw a frame with the given parameters for the given filter chain.
///
/// ## Safety
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_gl_opt_t`
///    struct.
libra_error_t libra_gl_filter_chain_frame(libra_gl_filter_chain_t *chain,
                                          size_t frame_count,
                                          struct libra_source_image_gl_t image,
                                          struct libra_viewport_t viewport,
                                          struct libra_draw_framebuffer_gl_t out,
                                          const float *mvp,
                                          const struct frame_gl_opt_t *opt);

/// Free a GL filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
libra_error_t libra_gl_filter_chain_free(libra_gl_filter_chain_t *chain);

/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
libra_error_t libra_d3d11_filter_chain_create(libra_shader_preset_t *preset,
                                              const struct filter_chain_d3d11_opt_t *options,
                                              const ID3D11Device *device,
                                              libra_d3d11_filter_chain_t *out);

/// Draw a frame with the given parameters for the given filter chain.
///
/// ## Safety
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_gl_opt_t`
///    struct.
libra_error_t libra_d3d11_filter_chain_frame(libra_d3d11_filter_chain_t *chain,
                                             size_t frame_count,
                                             struct libra_source_image_d3d11_t image,
                                             struct libra_viewport_t viewport,
                                             const ID3D11RenderTargetView *out,
                                             const float *mvp,
                                             const struct frame_d3d11_opt_t *opt);

/// Free a D3D11 filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
libra_error_t libra_d3d11_filter_chain_free(libra_d3d11_filter_chain_t *chain);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* __LIBRASHADER_H__ */
