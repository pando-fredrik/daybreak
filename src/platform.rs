use std::future::Future;
use glow::{Context};
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
mod abstraction {
    use std::time::Duration;
    use prokio::spawn_local;
    use prokio::time::sleep;
    use crate::debug_print;
    use super::*;

    pub type PlatformData = ();

    pub(crate) fn log(str: &str) {
        extern crate web_sys;
        use wasm_bindgen::JsValue;
        web_sys::console::log_1(&JsValue::from_str(str));
    }
    pub fn init() -> (Arc<Mutex<Context>>, &'static str, PlatformData) {
        use wasm_bindgen::JsCast;
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into().unwrap();

        let gl: web_sys::WebGl2RenderingContext = canvas.get_context("webgl2")
            .unwrap().unwrap().dyn_into().unwrap();

        let glow_ctx = glow::Context::from_webgl2_context(gl);
        (Arc::new(Mutex::new(glow_ctx)), "#version 300 es", ())
    }

    pub(crate) async fn run(platform_data: &mut PlatformData, context: &Arc<Mutex<Context>>, draw_callback: Arc<impl Fn(&Context, &str) + 'static>, shader_version: &str)
    {
        use prokio::spawn_local;
        use prokio::pinned::mpsc::unbounded;
        let (rx, tx) = unbounded::<()>();
        let async_context = context.clone();
        let shader_vers = shader_version.to_string();
        spawn_local(async move {
            loop {
                // Replace with request animation frame eventually... silly loop for now
                let ctx = async_context.lock().unwrap();
                draw_callback(&ctx, &shader_vers);
                sleep(Duration::from_millis(16)).await;
            }
        });

    }

    pub(crate) fn play_music(name: &str) {
        let audio = web_sys::HtmlAudioElement::new_with_src(name);
        let _ = audio.unwrap().play().unwrap();
    }

    pub(crate) fn rt(f: impl Future<Output = ()> + 'static) {
        spawn_local(f);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod abstraction {
    use super::*;
    use glutin::{self, ContextBuilder, ContextWrapper, event_loop::EventLoop, PossiblyCurrent};
    use std::sync::{Arc, Mutex};
    use glutin::window::Window;

    pub(crate) struct PlatformDataInner {
        pub(crate) event_loop: EventLoop<()>,
        pub(crate) window: ContextWrapper<PossiblyCurrent, Window>,
    }

    pub(crate) type PlatformData = Option<PlatformDataInner>;

    pub(crate) fn log(str: &str) {
        println!("{}", str);
    }
    pub(crate) unsafe fn init() -> (Arc<Mutex<Context>>, &'static str, Option<PlatformDataInner>) {
        let event_loop = EventLoop::new();
        let wb = glutin::window::WindowBuilder::new();
        let cb = ContextBuilder::new();
        let windowed_context = cb.build_windowed(wb, &event_loop).unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        let glow_ctx = Context::from_loader_function(|s| windowed_context.get_proc_address(s) as *const _);
        (Arc::new(Mutex::new(glow_ctx)), "#version 400", Some(PlatformDataInner {
            event_loop,
            window: windowed_context,
        }))
    }

    pub(crate) async fn run(platform_data: &mut PlatformData, context: &Arc<Mutex<Context>>, draw_callback: Arc<impl Fn(&Context, &str) + 'static>, shader_version: &str)
    {
        use glutin::event::{Event, WindowEvent};
        use glutin::event_loop::ControlFlow;

        let data = platform_data.take().unwrap();
        let win = data.window;
        let ev_loop = data.event_loop;

        let gl = context.clone();
        let callback = draw_callback.clone();
        let shader_version = shader_version.to_string();
        ev_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::LoopDestroyed => {}
                Event::MainEventsCleared => {
                    win.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let ctx = gl.lock().expect("Failed to lock context");
                    callback(&ctx, &shader_version);
                    win.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        win.resize(*physical_size);
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    _ => (),
                },
                _ => (),
            }
        });
    }

    pub(crate) fn play_music(filename: &str) {
        use std::fs::File;
        use std::io::BufReader;
        use std::thread;
        use rodio::{Decoder, OutputStream};

        let inner_file = filename.to_string();
        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let file = BufReader::new(File::open(inner_file).unwrap());
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            let source = Decoder::new(file).unwrap();
            sink.append(source);
            sink.sleep_until_end();
        });
    }

    pub(crate) fn rt(f: impl Future<Output = ()> + 'static) {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f);
    }
}

pub struct Platform {
    context: Arc<Mutex<Context>>,
    shader_version: &'static str,
    platform_data: abstraction::PlatformData,
}

#[macro_export]
macro_rules! debug_print {
    ( $( $t:tt )* ) => {
        Platform::log(&format!( $( $t )* ).as_str());
    }
}

impl Platform {

    pub fn log(str: &str) {
        abstraction::log(str);
    }
    pub fn new() -> Self {
        #[warn(unused_unsafe)]
        let (context, shader_version, platform_data) = unsafe { abstraction::init() };

        Self {
            context,
            shader_version,
            platform_data,
        }
    }
    pub fn play_music(&self, filename: &str)  {
        abstraction::play_music(filename);
    }

    pub async fn run(&mut self, draw_callback: Arc<impl Fn(&Context, &str) + 'static>) {
        abstraction::run(&mut self.platform_data, &self.context, draw_callback, self.shader_version).await;
    }

    pub fn with_gl<T>(&self, callback: impl Fn(&Context, &str) -> T) -> T {
        let context = self.context.lock().expect("Failed to lock context");
        callback(&context, self.shader_version)
    }

    pub fn spawn_rt(f: impl Future<Output = ()> + 'static) {
        abstraction::rt(f);
    }
}
