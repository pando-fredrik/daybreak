mod platform;

use std::sync::{Arc, Mutex};
use glow::*;
use crate::platform::Platform;
use glam::{Mat4, Vec3};

struct Uniforms {
    #[cfg(target_arch = "wasm32")]
    utime: Option<UniformLocation>,
    #[cfg(not(target_arch = "wasm32"))]
    utime: Option<NativeUniformLocation>,
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            utime: None,
        }
    }
}

fn main() {
    let platform = Arc::new(Mutex::new(Platform::new()));

    let program = {
        let platform_lock = platform.lock().unwrap();
        platform_lock.with_gl(|gl, _| unsafe {
            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vertex_array));

            gl.create_program().expect("Cannot create program")
        })
    };

    let (vertex_shader_source, fragment_shader_source) = (
        r#"
const vec3 verts[3] = vec3[3](
    vec3(0.5f, 0.5f, 0.0f),
    vec3(-0.5f, -0.5f, 0.0f),
    vec3(0.5f, -0.5f, 0.0f)
);

const vec3 colors[3] = vec3[3](
    vec3(1.0f, 0.0f, 0.0f),
    vec3(0.0f, 1.0f, 0.0f),
    vec3(0.0f, 0.0f, 1.0f)
);

out vec3 vertColor;
uniform float uTime;

mat4 rotateX(float theta) {
    float c = cos(theta);
    float s = sin(theta);
    return mat4(
        vec4(1, 0, 0, 0),
        vec4(0, c, -s, 0),
        vec4(0, s, c, 0),
        vec4(0, 0, 0, 1)
    );
}

// Funktion för att skapa en rotationsmatris runt Y-axeln
mat4 rotateY(float theta) {
    float c = cos(theta);
    float s = sin(theta);
    return mat4(
        vec4(c, 0, s, 0),
        vec4(0, 1, 0, 0),
        vec4(-s, 0, c, 0),
        vec4(0, 0, 0, 1)
    );
}

// Funktion för att skapa en rotationsmatris runt Z-axeln
mat4 rotateZ(float theta) {
    float c = cos(theta);
    float s = sin(theta);
    return mat4(
        vec4(c, -s, 0, 0),
        vec4(s, c, 0, 0),
        vec4(0, 0, 1, 0),
        vec4(0, 0, 0, 1)
    );
}

void main() {
    mat4 Zrot = rotateZ(uTime);
    mat4 Yrot = rotateY(uTime*.5);
    mat4 Xrot = rotateX(uTime);

    mat4 rotation = Zrot*Yrot*Xrot;
    vertColor = colors[gl_VertexID];
    vec3 vertex = verts[gl_VertexID];
    gl_Position = rotation * vec4(vertex, 1.0);
}
"#,
        r#"precision mediump float;
            in vec3 vertColor;
            out vec4 color;
            void main() {
                color = vec4(vertColor, 1.0);
            }"#,
    );

    let shader_sources = [
        (VERTEX_SHADER, vertex_shader_source),
        (FRAGMENT_SHADER, fragment_shader_source),
    ];


    let uniforms = Arc::new(Mutex::new(Uniforms::new()));

    let shaders = Arc::new(Mutex::new(Vec::with_capacity(shader_sources.len())));

    {
        let platform_lock = platform.lock().unwrap();
        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = platform_lock.with_gl(|gl, shader_version| unsafe {
                let shader = gl.create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shader
            });
            shaders.lock().expect("Failed to lock shader list").push(shader);
        }

        let platform_uniforms = uniforms.clone();
        platform_lock.with_gl(move |gl, _| unsafe {
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }
            debug_print!("Program linked, extracting uniforms");

            let mut uniforms = platform_uniforms.lock().unwrap();
            uniforms.utime = gl.get_uniform_location(program, "uTime");
            debug_print!("Done...");
        });
    }
    let local_platform = platform.clone();
    debug_print!("Setting up render loop");

    let program_uniforms = uniforms.clone();
    Platform::spawn_rt(async move {
        let ap = local_platform.clone();
        debug_print!("Locking platform");

        let mut platform_lock = ap.lock().unwrap();
        platform_lock.play_music("daybreak.mp3").await;

        debug_print!("Locked");

        let platform_uniforms = program_uniforms.clone();
        let start = instant::Instant::now();
        platform_lock.run(Arc::new(Box::new(move |gl: &Context, _shader_version: &str| unsafe {
            let now = instant::Instant::now();
            let time = now.duration_since(start);
            let time_f32: f32 = time.as_secs_f32();
            debug_print!("Render loop {:?}", time_f32);

            gl.use_program(Some(program));
            {
                debug_print!("Getting uniform locations");
                let uniforms = platform_uniforms.lock().unwrap();
                debug_print!("Setting uniforms");
                gl.uniform_1_f32(uniforms.utime.as_ref(), time_f32);
            }
            debug_print!("Clearing {}", time_f32%18.0);
            if (time_f32 > 6.2 && time_f32 < 6.3) || (time_f32 > 18.0 && time_f32 < 18.1) {
                gl.clear_color(1.0, 1.0, 1.0, 1.0);
            } else {
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
            }
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }))).await;
    });
}
