#ifndef __LIBRASHADER_LD_H__
#define __LIBRASHADER_LD_H__
#pragma once
#define LIBRA_RUNTIME_OPENGL
#define LIBRA_RUNTIME_D3D11
#define LIBRA_RUNTIME_VULKAN

#include "librashader.h"
#define _WIN32

#if defined(_WIN32)
#include <libloaderapi.h>
#elif defined(__linux__)
#include <dlfcn.h>
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

    PFN_libra_gl_init_context gl_init_context;
    PFN_libra_gl_filter_chain_create gl_filter_chain_create;
    PFN_libra_gl_filter_chain_frame gl_filter_chain_frame;
    PFN_libra_gl_filter_chain_free gl_filter_chain_free;

    PFN_libra_d3d11_filter_chain_create d3d11_filter_chain_create;
    PFN_libra_d3d11_filter_chain_frame d3d11_filter_chain_frame;
    PFN_libra_d3d11_filter_chain_free d3d11_filter_chain_free;

    PFN_libra_vk_filter_chain_create vk_filter_chain_create;
    PFN_libra_vk_filter_chain_frame vk_filter_chain_frame;
    PFN_libra_vk_filter_chain_free vk_filter_chain_free;
} libra_instance_t;

libra_instance_t librashader_load_instance(const char *filename) {
}

#endif