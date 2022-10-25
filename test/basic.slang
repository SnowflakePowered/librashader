#version 450
// 450 or 310 es are recommended

layout(set = 0, binding = 0, std140) uniform UBO
{
   mat4 MVP;
   vec4 SourceSize; // Not used here, but doesn't hurt
   float ColorMod;
};

#pragma name StockShader
#pragma format R8G8B8A8_UNORM
#pragma parameter ColorMod "Color intensity" 1.0 0.1 2.0 0.1

#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;
void main()
{
   gl_Position = MVP * Position;
   vTexCoord = TexCoord;
}

#pragma stage fragment
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
layout(binding = 1) uniform sampler2D Source;
// layout(binding = 1) uniform texture2D Source;
// layout(binding = 2) uniform sampler _Source_sampler;
void main()
{
   // FragColor = texture(sampler2D(Source, _Source_sampler), vTexCoord) * ColorMod;
   FragColor = texture(Source, vTexCoord) * ColorMod;
}