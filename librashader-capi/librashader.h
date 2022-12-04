#ifndef __LIBRASHADER_H__
#define __LIBRASHADER_H__

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct _filter_chain_gl _filter_chain_gl;

typedef struct _libra_error _libra_error;

/**
 * A shader preset including all specified parameters, textures, and paths to specified shaders.
 *
 * A shader preset can be used to create a filter chain runtime instance, or reflected to get
 * parameter metadata.
 */
typedef struct _shader_preset _shader_preset;

typedef const struct _libra_error *libra_error_t;

typedef struct _shader_preset *libra_shader_preset_t;

typedef const void *(*gl_loader_t)(const char*);

typedef struct filter_chain_gl_opt_t {
  uint16_t gl_version;
  bool use_dsa;
} filter_chain_gl_opt_t;

typedef struct _filter_chain_gl *libra_gl_filter_chain_t;

typedef struct libra_source_image_gl_t {
  uint32_t handle;
  uint32_t format;
  uint32_t width;
  uint32_t height;
} libra_source_image_gl_t;

typedef struct libra_viewport_t {
  float x;
  float y;
  uint32_t width;
  uint32_t height;
} libra_viewport_t;

typedef struct libra_draw_framebuffer_gl_t {
  uint32_t handle;
  uint32_t texture;
  uint32_t format;
  uint32_t width;
  uint32_t height;
} libra_draw_framebuffer_gl_t;

/**
 * Load a preset.
 */
typedef libra_error_t (*PFN_lbr_load_preset)(const char*, libra_shader_preset_t*);

typedef libra_error_t (*PFN_lbr_preset_free)(libra_shader_preset_t*);

typedef libra_error_t (*PFN_lbr_preset_set_param)(libra_shader_preset_t*, const char*, float);

typedef libra_error_t (*PFN_lbr_preset_get_param)(libra_shader_preset_t*, const char*, float*);

typedef libra_error_t (*PFN_lbr_preset_print)(libra_shader_preset_t*);

typedef libra_error_t (*PFN_lbr_preset_get_runtime_param_names)(libra_shader_preset_t*, float*);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

libra_error_t libra_load_preset(const char *filename, libra_shader_preset_t *out);

/**
 * Free the preset.
 */
libra_error_t libra_preset_free(libra_shader_preset_t *preset);

/**
 * Set the value of the parameter in the preset.
 */
libra_error_t libra_preset_set_param(libra_shader_preset_t *preset, const char *name, float value);

/**
 * Get the value of the parameter as set in the preset.
 */
libra_error_t libra_preset_get_param(libra_shader_preset_t *preset, const char *name, float *value);

/**
 * Pretty print the shader preset.
 */
libra_error_t libra_preset_print(libra_shader_preset_t *preset);

/**
 * Get a list of runtime parameter names.
 *
 * The returned value can not currently be freed.
 */
libra_error_t libra_preset_get_runtime_param_names(libra_shader_preset_t *preset,
                                                   const char **value);

/**
 * Initialize the OpenGL Context for librashader.
 *
 * ## Safety
 * Attempting to create a filter chain will fail.
 *
 * Reinitializing the OpenGL context with a different loader immediately invalidates previous filter
 * chain objects, and drawing with them causes immediate undefined behaviour.
 */
libra_error_t libra_gl_init_context(gl_loader_t loader);

/**
 * Create the filter chain given the shader preset.
 *
 * The shader preset is immediately invalidated and must be recreated after
 * the filter chain is created.
 *
 * ## Safety:
 * - `preset` must be either null, or valid and aligned.
 * - `options` must be either null, or valid and aligned.
 * - `out` may be either null or uninitialized, but must be aligned.
 */
libra_error_t libra_gl_filter_chain_create(libra_shader_preset_t *preset,
                                           const struct filter_chain_gl_opt_t *options,
                                           libra_gl_filter_chain_t *out);

libra_error_t libra_gl_filter_chain_frame(libra_gl_filter_chain_t *chain,
                                          size_t frame_count,
                                          struct libra_source_image_gl_t image,
                                          struct libra_viewport_t viewport,
                                          struct libra_draw_framebuffer_gl_t out,
                                          const float *mvp);

libra_error_t libra_gl_filter_chain_free(libra_gl_filter_chain_t *chain);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* __LIBRASHADER_H__ */
