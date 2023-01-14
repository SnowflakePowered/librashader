/*
librashader_ld.h
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

#ifndef __LIBRASHADER_LD_H__
#define __LIBRASHADER_LD_H__
#pragma once
#define LIBRA_RUNTIME_OPENGL
#define LIBRA_RUNTIME_D3D11
#define LIBRA_RUNTIME_VULKAN

#if defined(_WIN32)
#include <windows.h>
#elif defined(__linux__)
#include <dlfcn.h>
#endif

#include "librashader.h"

LIBRA_ERRNO __librashader__noop_error_errno(libra_error_t error) {
    return LIBRA_ERRNO_UNKNOWN_ERROR;
}

int32_t __librashader__noop_error_print(libra_error_t error) { return 1; }

int32_t __librashader__noop_error_free(libra_error_t *error) { return 1; }

int32_t __librashader__noop_error_write(libra_error_t error, char **out) {
    return 1;
}

int32_t __librashader__noop_error_free_string(char **out) { return 1; }

libra_error_t __librashader__noop_preset_create(const char *filename,
                                                libra_shader_preset_t *out) {
    return NULL;
}

libra_error_t __librashader__noop_preset_free(libra_shader_preset_t *preset) {
    return NULL;
}

libra_error_t __librashader__noop_preset_set_param(
    libra_shader_preset_t *preset, const char *name, float value) {
    return NULL;
}

libra_error_t __librashader__noop_preset_get_param(
    libra_shader_preset_t *preset, const char *name, float *value) {
    return NULL;
}

libra_error_t __librashader__noop_preset_print(libra_shader_preset_t *preset) {
    return NULL;
}

libra_error_t __librashader__noop_preset_get_runtime_param_names(
    libra_shader_preset_t *preset, const char **value) {
    return NULL;
}

#if defined(LIBRA_RUNTIME_OPENGL)
libra_error_t __librashader__noop_gl_init_context(libra_gl_loader_t loader) {
    return NULL;
}

libra_error_t __librashader__noop_gl_filter_chain_create(
    libra_shader_preset_t *preset, const struct filter_chain_gl_opt_t *options,
    libra_gl_filter_chain_t *out) {
    return NULL;
}

libra_error_t __librashader__noop_gl_filter_chain_frame(
    libra_gl_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_gl_t image, struct libra_viewport_t viewport,
    struct libra_draw_framebuffer_gl_t out, const float *mvp,
    const struct frame_gl_opt_t *opt) {
    return NULL;
}

libra_error_t __librashader__noop_gl_filter_chain_free(
    libra_gl_filter_chain_t *chain) {
    return NULL;
}
#endif

#if defined(LIBRA_RUNTIME_D3D11)
libra_error_t __librashader__noop_d3d11_filter_chain_create(
    libra_shader_preset_t *preset,
    const struct filter_chain_d3d11_opt_t *options, const ID3D11Device *device,
    libra_d3d11_filter_chain_t *out) {
    return NULL;
}

libra_error_t __librashader__noop_d3d11_filter_chain_frame(
    libra_d3d11_filter_chain_t *chain, size_t frame_count,
    struct libra_source_image_d3d11_t image, struct libra_viewport_t viewport,
    const ID3D11RenderTargetView *out, const float *mvp,
    const struct frame_vk_opt_t *opt) {
    return NULL;
}

libra_error_t __librashader__noop_d3d11_filter_chain_free(
    libra_d3d11_filter_chain_t *chain) {
    return NULL;
}
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
libra_error_t __librashader__noop_vk_filter_chain_create(
    struct libra_device_vk_t vulkan, libra_shader_preset_t *preset,
    const struct filter_chain_vk_opt_t *options, libra_vk_filter_chain_t *out) {
    return NULL;
}

libra_error_t __librashader__noop_vk_filter_chain_frame(
    libra_vk_filter_chain_t *chain, VkCommandBuffer command_buffer,
    size_t frame_count, struct libra_image_vk_t image,
    struct libra_viewport_t viewport, struct libra_image_vk_t out,
    const float *mvp, const struct FrameOptionsVulkan *opt) {
    return NULL;
}

libra_error_t __librashader__noop_vk_filter_chain_free(
    libra_vk_filter_chain_t *chain) {
    return NULL;
}
#endif

typedef struct libra_instance_t {
    PFN_libra_preset_create preset_create;
    PFN_libra_preset_free preset_free;
    PFN_libra_preset_set_param preset_set_param;
    PFN_libra_preset_get_param preset_get_param;
    PFN_libra_preset_print preset_print;
    PFN_libra_preset_get_runtime_param_names preset_get_runtime_param_names;

    PFN_libra_error_errno error_errno;
    PFN_libra_error_print error_print;
    PFN_libra_error_free error_free;
    PFN_libra_error_write error_write;
    PFN_libra_error_free_string error_free_string;

#if defined(LIBRA_RUNTIME_OPENGL)
    PFN_libra_gl_init_context gl_init_context;
    PFN_libra_gl_filter_chain_create gl_filter_chain_create;
    PFN_libra_gl_filter_chain_frame gl_filter_chain_frame;
    PFN_libra_gl_filter_chain_free gl_filter_chain_free;
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
    PFN_libra_d3d11_filter_chain_create d3d11_filter_chain_create;
    PFN_libra_d3d11_filter_chain_frame d3d11_filter_chain_frame;
    PFN_libra_d3d11_filter_chain_free d3d11_filter_chain_free;
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
    PFN_libra_vk_filter_chain_create vk_filter_chain_create;
    PFN_libra_vk_filter_chain_frame vk_filter_chain_frame;
    PFN_libra_vk_filter_chain_free vk_filter_chain_free;
#endif
} libra_instance_t;

libra_instance_t __librashader_make_null_instance() {
    return libra_instance_t{
        .preset_create = __librashader__noop_preset_create,
        .preset_free = __librashader__noop_preset_free,
        .preset_set_param = __librashader__noop_preset_set_param,
        .preset_get_param = __librashader__noop_preset_get_param,
        .preset_print = __librashader__noop_preset_print,
        .preset_get_runtime_param_names =
            __librashader__noop_preset_get_runtime_param_names,

        .error_errno = __librashader__noop_error_errno,
        .error_print = __librashader__noop_error_print,
        .error_free = __librashader__noop_error_free,
        .error_write = __librashader__noop_error_write,
        .error_free_string = __librashader__noop_error_free_string,

#if defined(LIBRA_RUNTIME_OPENGL)
        .gl_init_context = __librashader__noop_gl_init_context,
        .gl_filter_chain_create = __librashader__noop_gl_filter_chain_create,
        .gl_filter_chain_frame = __librashader__noop_gl_filter_chain_frame,
        .gl_filter_chain_free = __librashader__noop_gl_filter_chain_free,
#endif

#if defined(LIBRA_RUNTIME_D3D11)
        .d3d11_filter_chain_create =
            __librashader__noop_d3d11_filter_chain_create,
        .d3d11_filter_chain_frame =
            __librashader__noop_d3d11_filter_chain_frame,
        .d3d11_filter_chain_free = __librashader__noop_d3d11_filter_chain_free,
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
        .vk_filter_chain_create = __librashader__noop_vk_filter_chain_create,
        .vk_filter_chain_frame = __librashader__noop_vk_filter_chain_frame,
        .vk_filter_chain_free = __librashader__noop_vk_filter_chain_free,
#endif
    };
}

/// Load an instance of librashader in the OS-dependent search path of the
/// current directory.
///
/// `librashader_load_instance` loads from `librashader.dll` on Windows,
/// or `librashader.so` on Linux.
///
/// If no librashader implementation is found, the returned `libra_instance_t`
/// will have all function pointers set to no-op functions.
///
/// If any symbol fails to load, the function will be set to a no-op function.
///
/// \return An `libra_instance_t` struct with loaded function pointers.
libra_instance_t librashader_load_instance();

#if defined(_WIN32)
#define _LIBRASHADER_ASSIGN_FARPROC(HMOD, INSTANCE, NAME)                    \
    {                                                           \
        FARPROC address = GetProcAddress(HMOD, "libra_" #NAME); \
        if (address != NULL) {                                  \
            (INSTANCE).NAME = (PFN_libra_##NAME)address;        \
        }                                                       \
    }

libra_instance_t librashader_load_instance() {
    HMODULE librashader = LoadLibraryW(L"librashader.dll");
    libra_instance_t instance = __librashader_make_null_instance();
    if (librashader == 0) {
        return instance;
    }

    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_create);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_free);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_set_param);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_get_param);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_print);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, preset_get_runtime_param_names);

    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, error_errno);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, error_print);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, error_free);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, error_write);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, error_free_string);

#if defined(LIBRA_RUNTIME_OPENGL)
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, gl_init_context);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, gl_filter_chain_create);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, gl_filter_chain_frame);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, gl_filter_chain_free);
#endif

#if defined(LIBRA_RUNTIME_D3D11)
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, d3d11_filter_chain_create);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, d3d11_filter_chain_frame);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, d3d11_filter_chain_free);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, vk_filter_chain_create);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, vk_filter_chain_frame);
    _LIBRASHADER_ASSIGN_FARPROC(librashader, instance, vk_filter_chain_free);
#endif

    return instance;
}
#elif defined(__linux__)
#define _LIBRASHADER_ASSIGN_DLSYM(HMOD, INSTANCE, NAME)         \
    {                                                           \
        void* address = dlsym(HMOD, "libra_" #NAME); \
        if (address != NULL) {                                  \
            (INSTANCE).NAME = (PFN_libra_##NAME)address;        \
        }                                                       \
    }

libra_instance_t librashader_load_instance() {
    void* librashader = dlopen(L"librashader.so", RTLD_LAZY);
    libra_instance_t instance = __librashader_make_null_instance();
    if (librashader == NULL) {
        return instance;
    }

    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_create);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_free);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_set_param);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_get_param);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_print);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, preset_get_runtime_param_names);

    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, error_errno);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, error_print);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, error_free);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, error_write);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, error_free_string);

#if defined(LIBRA_RUNTIME_OPENGL)
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, gl_init_context);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, gl_filter_chain_create);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, gl_filter_chain_frame);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, gl_filter_chain_free);
#endif

    // Not sure why you would want this
#if defined(LIBRA_RUNTIME_D3D11)
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, d3d11_filter_chain_create);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, d3d11_filter_chain_frame);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, d3d11_filter_chain_free);
#endif

#if defined(LIBRA_RUNTIME_VULKAN)
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, vk_filter_chain_create);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, vk_filter_chain_frame);
    _LIBRASHADER_ASSIGN_DLSYM(librashader, instance, vk_filter_chain_free);
#endif
    return instance;
}
#else
libra_instance_t librashader_load_instance() {
    return __librashader_make_null_instance();
}
#endif
#endif