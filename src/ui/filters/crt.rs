use super::{
    Filter, FilterContext, FilterUniforms, Preprocessor, Program, TextureFilter, TextureFormat,
    TextureParams,
};

pub struct CrtFilter {
    program: Program<'static>,
    width: u32,
    height: u32,
    frame: Vec<u8>,
}

impl CrtFilter {
    pub fn new() -> Self {
        let width = 36 * 8;
        let height = 28 * 8;

        let processor = Preprocessor::new(super::CRT_SHADER);
        let program = processor.process().expect("valid shader source");

        Self {
            program,
            width,
            height,
            frame: vec![0; (width * height) as usize * 4],
        }
    }
}

impl<C: FilterContext> Filter<C> for CrtFilter {
    fn dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 2)
    }

    fn vertex_shader(&self) -> &str {
        &self.program.vertex
    }

    fn fragment_shader(&self) -> &str {
        &self.program.fragment
    }

    fn process(&mut self, display: &C, render_size: (f64, f64), screen: &[u8]) -> C::Uniforms {
        let mut unis = display.create_uniforms();

        let (frame, _) = self.frame.as_chunks_mut::<4>();
        let (screen, _) = screen.as_chunks::<3>();
        for (frame, screen) in frame.iter_mut().zip(screen.iter()) {
            frame[0] = screen[0];
            frame[1] = screen[1];
            frame[2] = screen[2];
            frame[3] = 0xff;
        }

        let params = TextureParams {
            width: self.width as usize,
            height: self.height as usize,
            format: TextureFormat::RGBA,
            pixels: &self.frame,
            filter: TextureFilter::Nearest,
        };

        let texture = display.create_texture(params);

        let s_w = self.width as f32;
        let s_h = self.height as f32;
        let d_w = render_size.0 as f32;
        let d_h = render_size.1 as f32;

        unis.add_texture("Source", texture);
        unis.add_vec4("SourceSize", (s_w, s_h, 1.0 / s_w, 1.0 / s_h));
        unis.add_vec4("OriginalSize", (s_w, s_h, 1.0 / s_w, 1.0 / s_h));
        unis.add_vec4("OutputSize", (d_w, d_h, 1.0 / d_w, 1.0 / d_h));

        for p in self.program.parameters.iter() {
            unis.add_f32(p.name, p.value);
        }

        unis
    }
}
