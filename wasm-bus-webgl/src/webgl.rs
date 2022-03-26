use std::sync::Arc;
pub use wasm_bus::prelude::CallError;

pub use super::api::glenum::*;

use super::api;
use super::error::WebGlError;

#[derive(Clone)]
pub struct WebGl
{
    raster: Arc<dyn api::Raster + Send + Sync + 'static>,
    ctx: Arc<dyn api::RenderingContext + Send + Sync + 'static>,
}

impl WebGl
{
    pub fn new() -> Result<WebGl, WebGlError> {
        let webgl = api::WebGlClient::new("os");
        let ctx = webgl.blocking_context()
            .map_err(convert_err)?;
        let raster = ctx.blocking_raster()
            .map_err(convert_err)?;
        
        Ok(
            WebGl {
                raster,
                ctx
            }
        )
    }

    pub fn create_program(&self) -> Program {
        Program {
            program: self.ctx.blocking_create_program().unwrap()
        }
    }

    pub fn create_buffer(&self) -> Buffer {
        Buffer {
            buffer: self.ctx.blocking_create_buffer().unwrap()
        }
    }

    pub fn create_vertex_array(&self) -> VertexArray {
        VertexArray {
            vertex_array: self.ctx.blocking_create_vertex_array().unwrap()
        }
    }

    pub fn create_texture(&self) -> Texture {
        Texture {
            texture: self.ctx.blocking_create_texture().unwrap()
        }
    }

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.raster.blocking_clear_color(red, green, blue, alpha).unwrap();
    }
    
    pub fn clear(&self, bit: BufferBit) {
        self.raster.blocking_clear(bit).unwrap();
    }

    pub fn clear_depth(&self, value: f32) {
        self.raster.blocking_clear_depth(value).unwrap();
    }

    pub fn draw_arrays(&self, mode: Primitives, first: i32, count: i32) {
        self.raster.blocking_draw_arrays(mode, first, count).unwrap();
    }

    pub fn draw_elements(&self, mode: Primitives, count: i32, kind: DataType, offset: u32) {
        self.raster.blocking_draw_elements(mode, count, kind, offset).unwrap();
    }

    pub fn enable(&self, flag: Flag) {
        self.raster.blocking_enable(flag).unwrap();
    }

    pub fn disable(&self, flag: Flag) {
        self.raster.blocking_disable(flag).unwrap();
    }

    pub fn cull_face(&self, culling: Culling) {
        self.raster.blocking_cull_face(culling).unwrap();
    }

    pub fn depth_mask(&self, val: bool) {
        self.raster.blocking_depth_mask(val).unwrap();
    }

    pub fn depth_funct(&self, val: DepthTest) {
        self.raster.blocking_depth_funct(val).unwrap();
    }

    pub fn viewport(&self, x: i32, y: i32, width: u32, height: u32) {
        self.raster.blocking_viewport(x, y, width, height).unwrap();
    }

    pub fn buffer_data(&self, kind: BufferKind, data: Vec<u8>, draw: DrawMode) {
        self.raster.blocking_buffer_data(kind, data, draw).unwrap();
    }

    pub fn buffer_data_f32(&self, kind: BufferKind, data: &[f32], draw: DrawMode) {
        let data = data.iter().flat_map(|a| a.to_ne_bytes()).collect();
        self.buffer_data(kind, data, draw);
    }

    pub fn unbind_buffer(&self, kind: BufferKind) {
        self.raster.blocking_unbind_buffer(kind).unwrap();
    }

    pub fn read_pixels(&self, x: u32, y: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType) -> Result<Vec<u8>, WebGlError> {
        self.raster.blocking_read_pixels(x, y, width, height, format, kind)
            .map_err(convert_err)
    }

    pub fn pixel_storei(&self, storage: PixelStorageMode, value: i32) {
        self.raster.blocking_pixel_storei(storage, value).unwrap();
    }

    pub fn generate_mipmap(&self) {
        self.raster.blocking_generate_mipmap().unwrap();
    }

    pub fn generate_mipmap_cube(&self) {
        self.raster.blocking_generate_mipmap_cube().unwrap();
    }

    pub fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: &[u8]) {
        let pixels = pixels.to_vec();
        self.raster.blocking_tex_image2d(target, level, width, height, format, kind, pixels).unwrap();
    }

    pub fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u32, yoffset: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>) {
        let pixels = pixels.to_vec();
        self.raster.blocking_tex_sub_image2d(target, level, xoffset, yoffset, width, height, format, kind, pixels).unwrap();
    }

    pub fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u32, height: u32, pixels: Vec<u8>) {
        let pixels = pixels.to_vec();
        self.raster.blocking_compressed_tex_image2d(target, level, compression, width, height, pixels).unwrap();
    }

    pub fn unbind_texture(&self, active: u32) {
        self.raster.blocking_unbind_texture(active).unwrap();
    }

    pub fn unbind_texture_cube(&self, active: u32) {
        self.raster.blocking_unbind_texture_cube(active).unwrap();
    }

    pub fn blend_equation(&self, eq: BlendEquation) {
        self.raster.blocking_blend_equation(eq).unwrap();
    }

    pub fn blend_func(&self, b1: BlendMode, b2: BlendMode) {
        self.raster.blocking_blend_func(b1, b2).unwrap();
    }

    pub fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.raster.blocking_blend_color(red, green, blue, alpha).unwrap();
    }

    pub fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, val: i32) {
        self.raster.blocking_tex_parameteri(kind, pname, val).unwrap();
    }

    pub fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, val: f32) {
        self.raster.blocking_tex_parameterfv(kind, pname, val).unwrap();
    }

    pub fn draw_buffers(&self, buffers: &[ColorBuffer]) {
        let buffers = buffers.to_vec();
        self.raster.blocking_draw_buffers(buffers).unwrap();
    }

    pub fn create_framebuffer(&self) -> FrameBuffer {
        FrameBuffer {
            framebuffer: self.raster.blocking_create_framebuffer().unwrap()
        }
    }

    pub fn unbind_framebuffer(&self, buffer: Buffers) {
        self.raster.blocking_unbind_framebuffer(buffer).unwrap();
    }

    pub fn unbind_vertex_array(&self) {
        self.raster.blocking_unbind_vertex_array().unwrap();
    }

    pub async fn sync(&self) {
        self.raster.sync().await.unwrap();
    }
}

#[derive(Clone)]
pub struct Buffer
{
    buffer: Arc<dyn api::Buffer + Send + Sync>,
}

impl Buffer
{
    pub fn bind(&self, kind: BufferKind) {
        self.buffer.blocking_bind_buffer(kind).unwrap();
    }
}

#[derive(Clone)]
pub struct VertexArray
{
    vertex_array: Arc<dyn api::VertexArray + Send + Sync>,
}

impl VertexArray
{
    pub fn bind(&self) {
        self.vertex_array.blocking_bind_vertex_array().unwrap();
    }

    pub fn unbind(&self) {
        self.vertex_array.blocking_unbind_vertex_array().unwrap();
    }
}

#[derive(Clone)]
pub struct Texture
{
    texture: Arc<dyn api::Texture + Send + Sync>,
}

impl Texture
{
    pub fn active_texture(&self, active: u32) {
        self.texture.blocking_active_texture(active).unwrap();
    }

    pub fn bind_texture(&self, target: TextureKind) {
        self.texture.blocking_bind_texture(target).unwrap();
    }

    pub fn bind_texture_cube(&self, target: TextureKind) {
        self.texture.blocking_bind_texture_cube(target).unwrap();
    }

    pub fn framebuffer_texture2d(&self, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32) {
        self.texture.blocking_framebuffer_texture2d(target, attachment, textarget, level).unwrap();
    }
}

#[derive(Clone)]
pub struct FrameBuffer
{
    framebuffer: Arc<dyn api::FrameBuffer + Send + Sync>,
}

impl FrameBuffer
{
    pub fn bind_framebuffer(&self, buffer: Buffers) {
        self.framebuffer.blocking_bind_framebuffer(buffer).unwrap();
    }
}

#[derive(Clone)]
pub struct Program
{
    program: Arc<dyn api::Program + Send + Sync>,
}

impl Program
{
    pub fn create_shader(&self, kind: ShaderKind) -> Shader {
        Shader {
            shader: self.program.blocking_create_shader(kind).unwrap()
        }
    }

    pub fn link(&self) -> Result<(), WebGlError> {
        self.program.blocking_link_program()
            .map_err(convert_err)?
            .map_err(|err| WebGlError::LinkError(err))
    }

    pub fn use_program(&self) {
        self.program.blocking_use_program().unwrap();
    }

    pub fn get_attrib_location(&self, name: &str) -> ProgramLocation {
        ProgramLocation {
            location: self.program.blocking_get_attrib_location(name.to_string()).unwrap()
        }
    }

    pub fn get_uniform_location(&self, name: &str) -> UniformLocation {
        UniformLocation {
            location: self.program.blocking_get_uniform_location(name.to_string()).unwrap()
        }
    }
}

#[derive(Clone)]
pub struct UniformLocation
{
    location: Arc<dyn api::UniformLocation + Send + Sync>,
}

impl UniformLocation
{
    pub fn uniform_matrix_4fv(&self, transpose: bool, value: [[f32; 4]; 4]) {
        self.location.blocking_uniform_matrix_4fv(transpose, value).unwrap();
    }

    pub fn uniform_matrix_3fv(&self, transpose: bool, value: [[f32; 3]; 3]) {
        self.location.blocking_uniform_matrix_3fv(transpose, value).unwrap();
    }

    pub fn uniform_matrix_2fv(&self, transpose: bool, value: [[f32; 2]; 2]) {
        self.location.blocking_uniform_matrix_2fv(transpose, value).unwrap();
    }

    pub fn uniform_1i(&self, value: i32) {
        self.location.blocking_uniform_1i(value).unwrap();
    }

    pub fn uniform_1f(&self, value: f32) {
        self.location.blocking_uniform_1f(value).unwrap();
    }

    pub fn uniform_2f(&self, v1: f32, v2: f32) {
        let value = (v1, v2);
        self.location.blocking_uniform_2f(value).unwrap();
    }

    pub fn uniform_3f(&self, v1: f32, v2: f32, v3: f32) {
        let value = (v1, v2, v3);
        self.location.blocking_uniform_3f(value).unwrap();
    }

    pub fn uniform_4f(&self, v1: f32, v2: f32, v3: f32, v4: f32) {
        let value = (v1, v2, v3, v4);
        self.location.blocking_uniform_4f(value).unwrap();
    }
}

#[derive(Clone)]
pub struct ProgramLocation
{
    location: Arc<dyn api::ProgramLocation + Send + Sync>,
}

impl ProgramLocation
{
    pub fn bind(&self)
    {
        self.location.blocking_bind_program_location().unwrap();
    }

    pub fn vertex_attrib_pointer(&self, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32)
    {
        self.location.blocking_vertex_attrib_pointer(size, kind, normalized, stride, offset).unwrap();
    }

    pub fn enable(&self)
    {
        self.location.blocking_enable_vertex_attrib_array().unwrap();
    }
}

#[derive(Clone)]
pub struct Shader
{
    shader: Arc<dyn api::Shader + Send + Sync>,
}

impl Shader
{
    pub fn set_source(&self, source: &str) {
        let source = source.to_string();
        self.shader.blocking_shader_source(source).unwrap();
    }

    pub fn compile(&self) -> Result<(), WebGlError> {
        self.shader.blocking_shader_compile()
            .map_err(convert_err)?
            .map_err(|err| WebGlError::CompileError(err))
    }

    pub fn attach(&self) -> Result<(), WebGlError> {
        self.shader.blocking_attach_shader()
            .map_err(convert_err)?
            .map_err(|err| WebGlError::LinkError(err))
    }
}

fn convert_err(err: CallError) -> WebGlError {
    WebGlError::IO(err.into_io_error())
}