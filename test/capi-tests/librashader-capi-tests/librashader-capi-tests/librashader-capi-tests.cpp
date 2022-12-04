// librashader-capi-tests.cpp : This file contains the 'main' function. Program execution begins and ends there.
//

#include <iostream>
#include <filesystem>

#include "librashader.h"
int main()
{
    std::cout << "Hello World!\n";
    std::cout << std::filesystem::current_path() << std::endl;
    libra_shader_preset_t preset;
    auto error = libra_load_preset("../../../slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp", &preset);
    if (error != NULL) {
        std::cout << "error happened\n";
    }
    libra_preset_print(&preset);
    libra_gl_filter_chain_t chain;
    error = libra_gl_create_filter_chain(&preset, NULL, &chain);
    if (error != NULL) {
        std::cout << "error happened\n";
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
