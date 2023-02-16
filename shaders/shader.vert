#version 450

layout (location=0) out vec4 data_from_the_vertexshader;

void main() {
    gl_PointSize=200.0;
    gl_Position = vec4(0.0,0.0,0.0,1.0);
    data_from_the_vertexshader = vec4(0.0,0.6,1.0,1.0);
}
