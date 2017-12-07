//! Full-screen pixel rate
//! Based on a glutin sample

extern crate gl;
extern crate glutin;

use gl::types::*;
use glutin::GlContext;

// Shader sources
static VS_SRC: &'static str = "
    #version 150 core

    void main() {
        switch (gl_VertexID) {
            case 0: gl_Position = vec4(-1.0, -3.0, 0.0, 1.0); break;
            case 1: gl_Position = vec4(3.0, 1.0, 0.0, 1.0);   break;
            case 2: gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);  break;
            default: gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
        }
    }"
;

static FS_SRC: &'static str = "
    #version 150 core
    out vec4 o_Color;

    void main() {
        o_Color = vec4(1.0, 1.0, 1.0, 1.0);
    }"
;

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    use std::ffi::CString;
    use std::ptr;
    unsafe {
        let shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let cs = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &cs.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        assert_eq!(status, 1);
        shader
    }
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
        assert_eq!(status, 1);
        program
    }
}

fn run_tests(
    test_name: &str,
    queries: &[GLuint],
    warmup: usize,
    gl_window: &glutin::GlWindow,
) {
    for &query in queries {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::BeginQuery(gl::TIME_ELAPSED, query);

            gl::DrawArrays(gl::TRIANGLES, 0, 3);

            gl::EndQuery(gl::TIME_ELAPSED);
            debug_assert_eq!(gl::GetError(), 0);
        }

        gl_window.swap_buffers().unwrap();
    }

    let total: usize = queries[warmup .. queries.len() - warmup]
        .iter()
        .map(|&query| unsafe {
            let mut result = 0;
            gl::GetQueryObjectuiv(query, gl::QUERY_RESULT, &mut result);
            result as usize
        })
        .sum();

    let (width, height) = gl_window.get_inner_size().unwrap();
    let pixel_count = (width * height) as usize;
    println!("Tested '{}' with {} samples", test_name, queries.len());
    let fullscreen_time = total / (queries.len() - 2 * warmup);
    println!("\tfull-screen time: {:.2} ms", fullscreen_time as f32 / 1.0e6);
    let megapixel_time = fullscreen_time * 1000 * 1000 / pixel_count;
    println!("\tmega-pixel time: {:.2} ms", megapixel_time as f32 / 1.0e6);
}

struct Config {
    with_color: bool,
    with_depth: bool,
    num_queries: usize,
    warmup_frames: usize,
}

fn main() {
    let config = Config {
        with_color: true,
        with_depth: true,
        num_queries: 100,
        warmup_frames: 20,
    };

    let events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("GL fill-rate benchmark")
        .with_fullscreen(Some(events_loop.get_primary_monitor()));
    let context = glutin::ContextBuilder::new()
        .with_vsync(false)
        .with_depth_buffer(24);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop)
        .unwrap();

    unsafe { gl_window.make_current() }.unwrap();

    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Create GLSL shaders
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);
    let mut queries = vec![0; config.num_queries];
    let mut vao = 0;

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenQueries(queries.len() as _, queries.as_mut_ptr());
        gl::BindVertexArray(vao);
        gl::UseProgram(program);
        assert_eq!(gl::GetError(), 0);
        gl::ClearColor(0.3, 0.3, 0.3, 1.0);
        gl::ClearDepth(1.0);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::TRUE);
    }

    let renderer_name = unsafe {
        use std::ffi::CStr;
        let ptr = gl::GetString(gl::RENDERER);
        CStr::from_ptr(ptr as _)
    };
    println!("Renderer: {:?}", renderer_name);
    let (width, height) = gl_window.get_inner_size().unwrap();
    println!("Screen: {}x{} resolution with {} hiDPI factor",
        width, height, gl_window.hidpi_factor());

    if config.with_color {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
        }
        run_tests(
            "color only",
            &queries,
            config.warmup_frames,
            &gl_window,
        );
    }

    if config.with_color && config.with_depth {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }
        run_tests(
            "color and depth",
            &queries,
            config.warmup_frames,
            &gl_window,
        );
    }

    unsafe {
        gl::DeleteProgram(program);
        gl::DeleteShader(fs);
        gl::DeleteShader(vs);
        gl::DeleteVertexArrays(1, &vao);
    }
}
