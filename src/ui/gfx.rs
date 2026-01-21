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

pub struct Gfx<const ROT_90: bool, T> {
    filter: T,
    display: GliumContext,
    indicies: glium::index::NoIndices,
    program: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    size: (f64, f64),
    frame: Frame,
    back_buffer: GfxBackBuffer,
}

impl<const ROT_90: bool, T: Filter<GliumContext>> Gfx<ROT_90, T> {
    pub fn new(display: Display<WindowSurface>, back_buffer: GfxBackBuffer, filter: T) -> Self {
        let shape = if ROT_90 {
            let top_right = Vertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 1.0],
            };
            let top_left = Vertex {
                position: [-1.0, 1.0],
                tex_coords: [1.0, 0.0],
            };
            let bottom_left = Vertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 0.0],
            };
            let bottom_right = Vertex {
                position: [1.0, -1.0],
                tex_coords: [0.0, 1.0],
            };
            [top_right, top_left, bottom_left, bottom_right]
        } else {
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
            [top_right, top_left, bottom_left, bottom_right]
        };

        let vertex_buffer = VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

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

        let size = filter.dimensions();
        let size = (size.0 as f64, size.1 as f64);

        Self {
            filter,
            display: GliumContext(display),
            indicies,
            program,
            vertex_buffer,
            size,
            back_buffer,
            frame: Frame::new(),
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.display.resize(size);
        let size = (size.0 as f64, size.1 as f64);
        self.size = size;
    }

    pub fn swap(&mut self) {
        self.back_buffer.swap(&mut self.frame);
    }

    pub fn render(&mut self) {
        let mut target = self.display.draw();

        let (filter_width, filter_height) = if ROT_90 {
            let (height, width) = self.filter.dimensions();
            (width, height)
        } else {
            self.filter.dimensions()
        };
        let (filter_width, filter_height) = (filter_width as f64, filter_height as f64);
        let (window_width, window_height) = self.size;
        let (surface_width, surface_height) = target.get_dimensions();
        let (surface_width, surface_height) = (surface_width as f64, surface_height as f64);
        let filter_ratio = filter_width / filter_height;
        let surface_ratio = surface_width / surface_height;

        let (left, bottom, width, height) = if filter_ratio > surface_ratio {
            let target_height = (1.0 / filter_ratio) * window_height;
            let target_height = (target_height / window_height) * surface_height * surface_ratio;
            (
                0,
                ((surface_height - target_height) / 2.0) as u32,
                surface_width as u32,
                target_height as u32,
            )
        } else {
            let target_width = (filter_ratio) * window_width;
            let target_width =
                (target_width / window_width) * surface_width * (1.0 / surface_ratio);
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

        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &self.vertex_buffer,
                &self.indicies,
                &self.program,
                &uniforms,
                &params,
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

enum FilterTexture {
    Texture2d(Texture2d),
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
    texture: FilterTexture,
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
