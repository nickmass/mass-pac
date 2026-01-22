mod crt;
mod pixelated;
mod preprocessor;

pub use crt::*;
pub use pixelated::*;
pub use preprocessor::*;

pub trait Filter<C: FilterContext> {
    fn dimensions(&self) -> (u32, u32);
    fn vertex_shader(&self) -> &str;
    fn fragment_shader(&self) -> &str;
    fn process(&mut self, ctx: &C, render_size: (f64, f64), screen: &[u8]) -> C::Uniforms;
}

impl<C: FilterContext> Filter<C> for Box<dyn Filter<C>> {
    fn dimensions(&self) -> (u32, u32) {
        self.as_ref().dimensions()
    }

    fn vertex_shader(&self) -> &str {
        self.as_ref().vertex_shader()
    }

    fn fragment_shader(&self) -> &str {
        self.as_ref().fragment_shader()
    }

    fn process(
        &mut self,
        ctx: &C,
        render_size: (f64, f64),
        screen: &[u8],
    ) -> <C as FilterContext>::Uniforms {
        self.as_mut().process(ctx, render_size, screen)
    }
}

pub trait FilterContext: Sized {
    type Uniforms: FilterUniforms<Self>;
    type Texture;
    fn create_uniforms(&self) -> Self::Uniforms;
    fn create_texture(&self, params: TextureParams) -> Self::Texture;
}

pub trait FilterUniforms<C: FilterContext> {
    fn add_f32(&mut self, name: &'static str, value: f32);
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32));
    fn add_vec4(&mut self, name: &'static str, value: (f32, f32, f32, f32));
    fn add_texture(&mut self, name: &'static str, value: C::Texture);
}

#[derive(Debug, Clone)]
pub struct TextureParams<'a> {
    pub width: usize,
    pub height: usize,
    pub format: TextureFormat,
    pub pixels: &'a [u8],
    pub filter: TextureFilter,
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFormat {
    RGBA,
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest,
}

pub(crate) const PIXELATED_SHADER: &'static str = include_str!("../../../shaders/pixelated.glsl");
pub(crate) const CRT_SHADER: &'static str = include_str!("../../../shaders/crt.glsl");
pub(crate) const TEXTURED_QUAD_SHADER: &'static str =
    include_str!("../../../shaders/textured_quad.glsl");
pub(crate) const PRELUDE_SHADER: &'static str = include_str!("../../../shaders/prelude_gl.glsl");
