// librashader-capi-tests.cpp : This file contains the 'main' function. Program execution begins and ends there.
//

#include <iostream>
#include <filesystem>

#define VK_DEFINE_HANDLE(object) typedef struct object##_T* object;
VK_DEFINE_HANDLE(VkInstance)
typedef void (*PFN_vkVoidFunction)(void);
typedef PFN_vkVoidFunction(*PFN_vkGetInstanceProcAddr)(VkInstance instance, const char* pName);


#define LIBRA_RUNTIME_OPENGL
#define LIBRA_RUNTIME_VULKAN

#include "../../../../librashader-capi/librashader.h"
int main()
{
    std::cout << "Hello World!\n";
    std::cout << std::filesystem::current_path() << std::endl;
    libra_shader_preset_t preset;
    auto error = libra_preset_create("../../../slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp", &preset);
    if (error != NULL) {
        std::cout << "error happened\n";
    }
    libra_preset_print(&preset);
    
    libra_gl_filter_chain_t chain;

    PFN_vkGetInstanceProcAddr GetInstanceProcAddr;
    libra_PFN_vkGetInstanceProcAddr entry = reinterpret_cast<libra_PFN_vkGetInstanceProcAddr>(GetInstanceProcAddr);

    error = libra_gl_filter_chain_create(NULL, NULL, &chain);
    if (error != NULL) {
        libra_error_print(error);
        char* error_str;
        libra_error_write(error, &error_str);
        printf("%s", error_str);
        libra_error_free_string(&error_str);
        printf("%s", error_str);
    }
    return 0;
}

// Run program: Ctrl + F5 or Debug > Start Without Debugging menu
// Debug program: F5 or Debug > Start Debugging menu

// Tips for Getting Started: 
//   1. Use the Solution Explorer window to add/manage files
//   2. Use the Team Explorer window to connect to source control
//   3. Use the Output window to see build output and other messages
//   4. Use the Error List window to view errors
//   5. Go to Project > Add New Item to create new code files, or Project > Add Existing Item to add existing code files to the project
//   6. In the future, to open this project again, go to File > Open > Project and select the .sln file
