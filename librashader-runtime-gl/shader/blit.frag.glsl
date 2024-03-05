#version 150

uniform sampler2D Texture;

in vec2 vTexCoord;
out vec4 FragColor;

void main()
{
    FragColor = vec4(texture(Texture, vTexCoord).rgb, 1.0);
}
