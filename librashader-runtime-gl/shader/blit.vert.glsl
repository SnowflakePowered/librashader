#version 150

uniform mat4 MVP;

in vec4 Position;
in vec2 TexCoord;
out vec2 vTexCoord;

void main()
{
    gl_Position = MVP * Position;
    vTexCoord = TexCoord;
}
