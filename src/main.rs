//! Full-screen pixel rate
//! Based on a glutin sample

#[macro_use]
extern crate bitflags;
extern crate gl;
extern crate glutin;

use gl::types::*;
use glutin::GlContext;
use std::ffi::CStr;

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

bitflags! {
    struct Flags: u32 {
        const CLEAR = 1 << 0;
        const DRAW = 1 << 1;
    }
}


fn run_tests(
    test_name: &str,
    clear_mask: GLenum,
    num_draws: usize,
    queries: &[GLuint],
    warmup: usize,
    flags: Flags,
    gl_window: &glutin::GlWindow,
    clear_scissored: bool,
    width: u32,
    height: u32,
) -> (usize, usize) {
    for &query in queries {
        unsafe {
            if flags.contains(Flags::CLEAR) {
                gl::BeginQuery(gl::TIME_ELAPSED, query);
            }
            if clear_scissored {
                gl::Enable(gl::SCISSOR_TEST);
                gl::Scissor(1, 1, (width / 2) as i32, (height / 2) as i32);
            }
            gl::Clear(clear_mask);
            if clear_scissored {
                gl::Disable(gl::SCISSOR_TEST);
            }
            if !flags.contains(Flags::CLEAR) {
                gl::BeginQuery(gl::TIME_ELAPSED, query);
            }
            if !flags.contains(Flags::DRAW) {
                gl::EndQuery(gl::TIME_ELAPSED);
            }

            gl::DrawArraysInstanced(gl::TRIANGLES, 0, 3, num_draws as _);

            if flags.contains(Flags::DRAW) {
                gl::EndQuery(gl::TIME_ELAPSED);
            }
            debug_assert_eq!(gl::GetError(), 0);
        }

        gl_window.swap_buffers().unwrap();
    }

    let total_time = queries[warmup .. queries.len() - warmup]
        .iter()
        .map(|&query| unsafe {
            let mut result = 0;
            gl::GetQueryObjectuiv(query, gl::QUERY_RESULT, &mut result);
            result as usize
        })
        .sum::<usize>();

    let (width, height) = gl_window.get_inner_size().unwrap();
    let hidpi = gl_window.hidpi_factor();
    let pixel_count = (width as f32 * height as f32 * hidpi) as usize;
    println!("Tested '{}' with {} samples of {} instances",
        test_name, queries.len(), num_draws);

    let total_draws = (queries.len() - 2 * warmup) * num_draws;
    let fullscreen_time = total_time / total_draws;
    println!("\tfull-screen time: {:.2} ms", fullscreen_time as f32 / 1.0e6);
    let megapixel_time = fullscreen_time * 1000 * 1000 / pixel_count;
    println!("\tmega-pixel time: {} mcs", megapixel_time / 1000);

    (fullscreen_time, megapixel_time)
}

struct Config {
    num_queries: usize,
    warmup_frames: usize,
    num_rejects: usize,
    clear_scissored: bool,
}

fn main() {
    let config = Config {
        num_queries: 200,
        warmup_frames: 40,
        num_rejects: 20,
        clear_scissored: false,
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
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::DepthMask(gl::TRUE);
    }

    let renderer_name = unsafe {
        CStr::from_ptr(gl::GetString(gl::RENDERER) as _)
    };
    let version_name = unsafe {
        CStr::from_ptr(gl::GetString(gl::VERSION) as _)
    };
    println!("Renderer: {:?}", renderer_name);
    println!("Version: {:?}", version_name);
    let (width, height) = gl_window.get_inner_size().unwrap();
    println!("Screen: {}x{} resolution with {} hiDPI factor",
        width, height, gl_window.hidpi_factor());

    let (fs_color, mp_color) = run_tests(
        "color and depth",
        gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT,
        1,
        &queries,
        config.warmup_frames,
        Flags::DRAW,
        &gl_window,
        config.clear_scissored,
        width,
        height,
    );

    unsafe {
        gl::Flush();
        gl::ClearColor(1.0, 0.3, 0.3, 1.0);
    }

    let (_, mp_depth_reject) = run_tests(
        "depth rejected",
        gl::COLOR_BUFFER_BIT,
        config.num_rejects,
        &queries,
        config.warmup_frames,
        Flags::DRAW,
        &gl_window,
        config.clear_scissored,
        width,
        height,
    );

    let (_, mp_color_clear) = run_tests(
        "depth rejected",
        gl::COLOR_BUFFER_BIT,
        config.num_rejects,
        &queries,
        config.warmup_frames,
        Flags::CLEAR,
        &gl_window,
        config.clear_scissored,
        width,
        height,
    );

    println!("Table entry:");
    println!("| {} | {:?} | {:?} | {}x{} | {} | {:.2} ms | {} mcs | {} mcs | {} mcs |",
        std::env::consts::OS, version_name, renderer_name,
        width, height, gl_window.hidpi_factor(),
        fs_color as f32 * 1.0e-6,
        mp_color_clear / 1000,
        mp_color / 1000,
        mp_depth_reject / 1000
    );

    unsafe {
        gl::DeleteProgram(program);
        gl::DeleteShader(fs);
        gl::DeleteShader(vs);
        gl::DeleteVertexArrays(1, &vao);
    }
}
