#version 140

uniform sampler2D tex;

in vec2 v_tex_pos;
in vec4 v_color;

out vec4 f_color;

void main() {
    float alpha = texture(tex, v_tex_pos).r;

    if (alpha <= 0.0) {
        discard;
    }

    f_color = v_color * vec4(1.0, 1.0, 1.0, alpha);
}
