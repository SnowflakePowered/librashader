#version 450
// 450 or 310 es are recommended

layout(set = 0, binding = 0, std140) uniform UBO
{
   mat4 MVP;
   float ColorMod;
   uint FrameCount;
};

layout(push_constant) uniform Push {
    float ColorMod2;
   float x_tilt;
   float y_tilt;
} registers;

#pragma name StockShader
#pragma format R8G8B8A8_UNORM
#pragma parameter ColorMod "Color intensity" 1.0 0.1 2.0 0.1
#pragma parameter ColorMod2 "Color intensity" 1.0 0.1 2.0 0.1

#pragma parameter x_tilt "CRTGeom Horizontal Tilt" 0.0 -0.5 0.5 0.05
#pragma parameter y_tilt "CRTGeom Vertical Tilt" 0.0 -0.5 0.5 0.05

#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;
layout(location = 1) out vec2 sinangle;
layout(location = 2) out vec3 stretch;

#define FIX(c) max(abs(c), 1e-5);

vec2 aspect = vec2(1.0, 0.75);

float intersect(vec2 xy)
{
   float A = dot(xy,xy) + 2.25;
   float B = 2.0*(2.0*(dot(xy,sinangle)-1.5)-2.25);

   return (-B-sqrt(B*B-4.0*A*8.25))/(2.0*A);
}

vec2 bkwtrans(vec2 xy)
{
   float c     = intersect(xy);
   vec2 point  = (vec2(c, c) * xy - vec2(-2.0, -2.0)*sinangle) / vec2(2.0, 2.0);

   float A     = dot(sinangle, sinangle) + 1.0;
   float B     = -2.0*dot(point, sinangle);
   float C     = dot(point, point) - 1.0;

   float a     = (-B + sqrt(B*B - 4.0*A*C))/(2.0*A);
   vec2 uv     = (point - a*sinangle);
   float r     = FIX(2.0*acos(a));

   return uv*r/sin(r/2.0);
}

vec2 fwtrans(vec2 uv)
{
   float r = FIX(sqrt(dot(uv,uv)));
   uv *= sin(r/2.0)/r;
   float x = 1.0-cos(r/2.0);
   float D = 1.5/2.0 + x+dot(uv,sinangle);

   return 1.5*(uv-x*sinangle)/D;
}

vec3 maxscale()
{
   vec2 c  = bkwtrans(-2.0 * sinangle / (1.0 + 2.0/1.5));
   vec2 a  = vec2(0.5,0.5)*aspect;

   vec2 lo = vec2(fwtrans(vec2(-a.x,  c.y)).x,
   fwtrans(vec2( c.x, -a.y)).y)/aspect;

   vec2 hi = vec2(fwtrans(vec2(+a.x,  c.y)).x,
   fwtrans(vec2( c.x, +a.y)).y)/aspect;

   return vec3((hi+lo)*aspect*0.5,max(hi.x-lo.x,hi.y-lo.y));
}

void main()
{
   gl_Position = MVP * Position;
   vTexCoord = TexCoord;
//   sinangle = sin(vec2(registers.x_tilt, registers.y_tilt));
//   sinangle = vec2(registers.x_tilt, registers.y_tilt);
      sinangle = vec2(0.0, 0.0);
   stretch = maxscale();
}

#pragma stage fragment
layout(location = 0) in vec2 vTexCoord;
layout(location = 1) in vec2 sinangle;
layout(location = 2) in vec3 stretch;

layout(location = 0) out vec4 FragColor;
layout(binding = 1) uniform sampler2D Source;
void main()
{
   FragColor = texture(Source, vTexCoord) * ColorMod * registers.ColorMod2;
}