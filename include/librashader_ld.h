#ifndef __LIBRASHADER_LD_H__
#define __LIBRASHADER_LD_H__
#pragma once
#include "librashader.h"

#if defined(_WIN32)
#include <libloaderapi.h>
#elif defined(__linux__)
#include <dlfcn.h>
#endif

typedef struct libra_instance_t {

};

#endif