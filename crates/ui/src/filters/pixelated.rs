use super::{
    Filter, FilterContext, FilterUniforms, Preprocessor, Program, TextureFilter, TextureFormat,
    TextureParams,
};

pub struct PixelatedFilter {
    program: Program<'static>,
    width: u32,
    height: u32,
    frame: Vec<u8>,
}

impl PixelatedFilter {
    pub fn new() -> PixelatedFilter {
        let width = 36 * 8;
        let height = 28 * 8;
        let processor = Preprocessor::new(super::PIXELATED_SHADER);
        let program = processor.process().expect("valid shader source");

        PixelatedFilter {
            program,
            width,
            height,
            frame: vec![0; (width * height) as usize * 4],
        }
    }
}

impl<C: FilterContext> Filter<C> for PixelatedFilter {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn vertex_shader(&self) -> &str {
        &self.program.vertex
    }

    fn fragment_shader(&self) -> &str {
        &self.program.fragment
    }

    fn process(&mut self, display: &C, render_size: (f64, f64), screen: &[u8]) -> C::Uniforms {
        let (frame, _) = self.frame.as_chunks_mut::<4>();
        let (screen, _) = screen.as_chunks::<3>();
        for (frame, screen) in frame.iter_mut().zip(screen.iter()) {
            frame[0] = screen[0];
            frame[1] = screen[1];
            frame[2] = screen[2];
            frame[3] = 0xff;
        }

        let mut uniforms = display.create_uniforms();

        let tex_params = TextureParams {
            width: self.width as usize,
            height: self.height as usize,
            format: TextureFormat::RGBA,
            pixels: &self.frame,
            filter: TextureFilter::Nearest,
        };

        let texture = display.create_texture(tex_params);

        uniforms.add_vec2("input_size", (self.width as f32, self.height as f32));
        uniforms.add_vec2("output_size", (render_size.0 as f32, render_size.1 as f32));
        uniforms.add_texture("screen", texture);

        uniforms
    }
}
