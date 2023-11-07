mod platform;

use std::sync::{Arc, Mutex};
use glow::*;
use crate::platform::Platform;

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
        r#"const vec2 verts[3] = vec2[3](
                vec2(0.5f, 1.0f),
                vec2(0.0f, 0.0f),
                vec2(1.0f, 0.0f)
            );
            out vec2 vert;
            void main() {
                vert = verts[gl_VertexID];
                gl_Position = vec4(vert - 0.5, 0.0, 1.0);
            }"#,
        r#"precision mediump float;
            in vec2 vert;
            out vec4 color;
            void main() {
                color = vec4(vert, 0.5, 1.0);
            }"#,
    );

    let shader_sources = [
        (VERTEX_SHADER, vertex_shader_source),
        (FRAGMENT_SHADER, fragment_shader_source),
    ];

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
        platform_lock.with_gl(|gl, _| unsafe {
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }
        });

        platform_lock.play_music("daybreak.mp3");
    }

    let local_platform = platform.clone();
    Platform::spawn_rt(async move {
        let ap = local_platform.clone();
        let mut platform_lock = ap.lock().unwrap();
        platform_lock.run(Arc::new(Box::new(move |gl: &Context, _shader_version: &str| unsafe {
            let time = instant::Instant::now();
            gl.use_program(Some(program));
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            debug_print!("Drawing {:?}", time);
        }))).await;
    });
}
