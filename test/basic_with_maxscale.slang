#version 450
// 450 or 310 es are recommended

layout(set = 0, binding = 0, std140) uniform UBO
{
   float x;
   float y;
};

layout(push_constant) uniform Push {
   float ColorMod2;
} registers;

#pragma name StockShader
#pragma format R8G8B8A8_UNORM
#pragma parameter ColorMod2 "Color intensity" 1.0 0.1 2.0 0.1

#pragma parameter x "CRTGeom Horizontal Tilt" 0.0 -0.5 0.5 0.05
#pragma parameter y "CRTGeom Vertical Tilt" 0.0 -0.5 0.5 0.05

#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 1) out float test;

void main()
{
   gl_Position = Position;
   vec2 vec = vec2(x, y);
// vec2 vec = vec2(0.0, 0.0);

   float A     = dot(vec, vec) + 1.0;
   float r     =  max(abs(2.0*acos(A)), 0.01);

   // This is inf when vec = vec2(x, y);
   // but 200 when vec2(0.0, 0.0);
   test = 1.0 / sin(r/2.0);
}

#pragma stage fragment
layout(location = 0) in float test;

layout(location = 0) out vec4 FragColor;
layout(binding = 1) uniform sampler2D Source;
void main()
{
   FragColor = texture(Source, vec2(0.0)) * registers.ColorMod2;
}