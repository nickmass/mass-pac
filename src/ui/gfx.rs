use glium::framebuffer::SimpleFrameBuffer;
use glium::glutin::surface::WindowSurface;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d};
use glium::uniforms::{SamplerWrapFunction, UniformValue, Uniforms};
use glium::winit::event_loop::EventLoopProxy;
use glium::{Display, Program, Surface, VertexBuffer, implement_vertex};

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use super::filters::{Filter, FilterContext, FilterUniforms, TextureFormat};

use super::app::UserEvent;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct Gfx<T> {
    filter: T,
    display: GliumContext,
    indices: glium::index::NoIndices,
    program: Program,
    simple_draw: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    rotated_vertex_buffer: VertexBuffer<Vertex>,
    frame: Frame,
    back_buffer: GfxBackBuffer,
    frame_buffer: glium::Texture2d,
}

impl<T: Filter<GliumContext>> Gfx<T> {
    pub fn new(display: Display<WindowSurface>, back_buffer: GfxBackBuffer, filter: T) -> Self {
        let top_right = Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 0.0],
        };
        let top_left = Vertex {
            position: [-1.0, 1.0],
            tex_coords: [0.0, 0.0],
        };
        let bottom_left = Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 1.0],
        };
        let bottom_right = Vertex {
            position: [1.0, -1.0],
            tex_coords: [1.0, 1.0],
        };
        let shape = [top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = VertexBuffer::new(&display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let top_right = Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 0.0],
        };
        let top_left = Vertex {
            position: [-1.0, 1.0],
            tex_coords: [1.0, 1.0],
        };
        let bottom_left = Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 1.0],
        };
        let bottom_right = Vertex {
            position: [1.0, -1.0],
            tex_coords: [0.0, 0.0],
        };
        let shape = [top_right, top_left, bottom_left, bottom_right];

        let rotated_vertex_buffer = VertexBuffer::new(&display, &shape).unwrap();

        let program = Program::from_source(
            &display,
            &*filter.vertex_shader(),
            &*filter.fragment_shader(),
            None,
        );

        let program = match program {
            Ok(p) => p,
            Err(glium::CompilationError(msg, kind)) => {
                panic!("Shader Compilation Error '{kind:?}':\n{msg}")
            }
            Err(e) => panic!("{e:?}"),
        };

        let quad_shaders = super::filters::Preprocessor::new(super::filters::TEXTURED_QUAD_SHADER)
            .process()
            .unwrap();
        let simple_draw = Program::from_source(
            &display,
            &*quad_shaders.vertex,
            &*quad_shaders.fragment,
            None,
        );

        let simple_draw = match simple_draw {
            Ok(p) => p,
            Err(glium::CompilationError(msg, kind)) => {
                panic!("Shader Compilation Error '{kind:?}':\n{msg}")
            }
            Err(e) => panic!("{e:?}"),
        };

        let (width, height) = display.get_framebuffer_dimensions();
        let frame_buffer = Texture2d::empty(&display, height, width).unwrap();

        Self {
            filter,
            display: GliumContext(display),
            indices,
            program,
            simple_draw,
            vertex_buffer,
            rotated_vertex_buffer,
            back_buffer,
            frame: Frame::new(),
            frame_buffer,
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.display.resize(size);
    }

    pub fn swap(&mut self) {
        self.back_buffer.swap(&mut self.frame);
    }

    pub fn render(&mut self) {
        let (surface_width, surface_height) = self.display.get_framebuffer_dimensions();
        // render at 90 degrees
        let (surface_width, surface_height) = (surface_height, surface_width);
        if (surface_width, surface_height) != self.frame_buffer.dimensions() {
            let Ok(frame_buffer) = Texture2d::empty(&*self.display, surface_width, surface_height)
            else {
                return;
            };
            self.frame_buffer = frame_buffer;
        }

        let (filter_width, filter_height) = self.filter.dimensions();

        let (filter_width, filter_height) = (filter_width as f64, filter_height as f64);
        let (surface_width, surface_height) = (surface_width as f64, surface_height as f64);
        let filter_ratio = filter_width / filter_height;
        let surface_ratio = surface_width / surface_height;

        let (left, bottom, width, height) = if filter_ratio > surface_ratio {
            let target_height = (1.0 / filter_ratio) * surface_height;
            let target_height = target_height * surface_ratio;
            (
                0,
                ((surface_height - target_height) / 2.0) as u32,
                surface_width as u32,
                target_height as u32,
            )
        } else {
            let target_width = (filter_ratio) * surface_width;
            let target_width = target_width * (1.0 / surface_ratio);
            (
                ((surface_width - target_width) / 2.0) as u32,
                0,
                target_width as u32,
                surface_height as u32,
            )
        };

        let uniforms =
            self.filter
                .process(&self.display, (width as f64, height as f64), &self.frame);

        let params = glium::DrawParameters {
            viewport: Some(glium::Rect {
                left,
                bottom,
                width,
                height,
            }),
            ..Default::default()
        };

        let Ok(mut frame_buffer) = SimpleFrameBuffer::new(&*self.display, &self.frame_buffer)
        else {
            return;
        };
        frame_buffer.clear_color(0.0, 0.0, 0.0, 1.0);
        frame_buffer
            .draw(
                &self.vertex_buffer,
                &self.indices,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();

        let mut target = self.display.draw();
        let mut uniforms = self.display.create_uniforms();
        uniforms.add_2d_uniform_ref("tex", &self.frame_buffer, FilterScaling::Nearest);

        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &self.rotated_vertex_buffer,
                &self.indices,
                &self.simple_draw,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
    }
}

pub struct Frame(Box<[u8]>);

impl Frame {
    pub fn new() -> Self {
        let inner = vec![0; (36 * 8) * (28 * 8) * 3].into_boxed_slice();
        Frame(inner)
    }
}

impl std::ops::Deref for Frame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Frame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct GfxBackBuffer {
    frame: Arc<Mutex<Frame>>,
    tx: EventLoopProxy<UserEvent>,
}

impl GfxBackBuffer {
    pub fn new(tx: EventLoopProxy<UserEvent>) -> Self {
        let frame = Arc::new(Mutex::new(Frame::new()));
        Self { frame, tx }
    }

    pub fn update<F: FnOnce(&mut [u8])>(&mut self, func: F) {
        {
            let mut frame = self.frame.lock().unwrap();
            func(&mut frame);
        }
        let _ = self.tx.send_event(UserEvent::Frame);
    }

    pub fn swap(&self, other: &mut Frame) {
        let mut frame = self.frame.lock().unwrap();
        std::mem::swap(&mut *frame, other);
    }
}

pub struct GliumContext(Display<WindowSurface>);

impl std::ops::Deref for GliumContext {
    type Target = Display<WindowSurface>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for GliumContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FilterContext for GliumContext {
    type Uniforms = UniformCollection<'static>;

    type Texture = Texture;

    fn create_uniforms(&self) -> Self::Uniforms {
        UniformCollection::new()
    }

    fn create_texture(&self, params: super::filters::TextureParams) -> Self::Texture {
        let filter = match params.filter {
            super::filters::TextureFilter::Nearest => FilterScaling::Nearest,
        };
        match params.format {
            f @ TextureFormat::RGBA => {
                let pixel_format = match f {
                    TextureFormat::RGBA => ClientFormat::U8U8U8U8,
                };

                let img = RawImage2d {
                    data: Cow::Borrowed(params.pixels),
                    width: params.width as u32,
                    height: params.height as u32,
                    format: pixel_format,
                };

                let tex = Texture2d::with_mipmaps(&self.0, img, MipmapsOption::NoMipmap).unwrap();

                Texture::Texture2d(tex, filter)
            }
        }
    }
}

impl FilterUniforms<GliumContext> for UniformCollection<'static> {
    fn add_f32(&mut self, name: &'static str, value: f32) {
        self.add(name, value);
    }

    fn add_vec2(&mut self, name: &'static str, value: (f32, f32)) {
        self.add(name, value);
    }

    fn add_vec4(&mut self, name: &'static str, value: (f32, f32, f32, f32)) {
        self.add(name, value);
    }

    fn add_texture(&mut self, name: &'static str, value: Texture) {
        match value {
            Texture::Texture2d(tex, scale) => self.add_2d_uniform(name, tex, scale),
        }
    }
}

pub enum Texture {
    Texture2d(Texture2d, FilterScaling),
}

pub enum FilterScaling {
    Nearest,
}

enum FilterTexture<'a> {
    Texture2d(Texture2d),
    Texture2dRef(&'a Texture2d),
}

enum FilterUniform<'a> {
    Sampler(FilterSampler<'a>),
    Simple {
        name: &'a str,
        value: UniformValue<'static>,
    },
}

pub struct FilterSampler<'a> {
    name: &'a str,
    texture: FilterTexture<'a>,
    scaling: FilterScaling,
}

pub struct UniformCollection<'a> {
    uniforms: Vec<FilterUniform<'a>>,
}

impl<'a> UniformCollection<'a> {
    pub fn new() -> Self {
        Self {
            uniforms: Vec::new(),
        }
    }

    pub fn add_2d_uniform(&mut self, name: &'a str, tex: Texture2d, scale: FilterScaling) {
        let uni = FilterSampler {
            name,
            texture: FilterTexture::Texture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(FilterUniform::Sampler(uni));
    }

    pub fn add_2d_uniform_ref(&mut self, name: &'a str, tex: &'a Texture2d, scale: FilterScaling) {
        let uni = FilterSampler {
            name,
            texture: FilterTexture::Texture2dRef(tex),
            scaling: scale,
        };

        self.uniforms.push(FilterUniform::Sampler(uni));
    }

    pub fn add<T: ToUniform>(&mut self, name: &'a str, value: T) {
        self.uniforms.push(FilterUniform::Simple {
            name: name.into(),
            value: value.to_uniform(),
        })
    }
}

impl<'a> Uniforms for UniformCollection<'a> {
    fn visit_values<'b, F: FnMut(&str, glium::uniforms::UniformValue<'b>)>(&'b self, mut visit: F) {
        use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
        for uni in self.uniforms.iter() {
            match uni {
                FilterUniform::Simple { name, value } => visit(&*name, value.clone()),
                FilterUniform::Sampler(uni) => {
                    let (mag_scale, min_scale) = match uni.scaling {
                        FilterScaling::Nearest => {
                            (MagnifySamplerFilter::Nearest, MinifySamplerFilter::Nearest)
                        }
                    };

                    let mut sampler = glium::uniforms::SamplerBehavior::default();
                    sampler.magnify_filter = mag_scale;
                    sampler.minify_filter = min_scale;
                    sampler.wrap_function = (
                        SamplerWrapFunction::Clamp,
                        SamplerWrapFunction::Clamp,
                        SamplerWrapFunction::Clamp,
                    );

                    match uni.texture {
                        FilterTexture::Texture2d(ref tex) => {
                            visit(uni.name, UniformValue::Texture2d(tex, Some(sampler)));
                        }
                        FilterTexture::Texture2dRef(tex) => {
                            visit(uni.name, UniformValue::Texture2d(tex, Some(sampler)));
                        }
                    }
                }
            }
        }
    }
}

pub trait ToUniform {
    fn to_uniform(self) -> UniformValue<'static>;
}

impl ToUniform for f32 {
    fn to_uniform(self) -> UniformValue<'static> {
        UniformValue::Float(self)
    }
}

impl ToUniform for (f32, f32) {
    fn to_uniform(self) -> UniformValue<'static> {
        UniformValue::Vec2([self.0, self.1])
    }
}

impl ToUniform for (f32, f32, f32, f32) {
    fn to_uniform(self) -> UniformValue<'static> {
        UniformValue::Vec4([self.0, self.1, self.2, self.3])
    }
}
