#version 140

in vec2 position;
in vec2 tex_pos;
in vec4 color;

out vec2 v_tex_pos;
out vec4 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_pos = tex_pos;
    v_color = color;
}
