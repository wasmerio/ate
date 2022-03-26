use std::sync::Arc;
#[allow(unused_imports)]
use wasm_bus::macros::*;

pub mod glenum;
pub use glenum::*;

#[wasm_bus(format = "bincode")]
pub trait WebGl {
    async fn context(&self) -> Arc<dyn RenderingContext>;
}

#[wasm_bus(format = "bincode")]
pub trait RenderingContext {
    async fn raster(&self) -> Arc<dyn Raster>;

    async fn create_program(&self) -> Arc<dyn Program>;

    async fn create_buffer(&self) -> Arc<dyn Buffer>;

    async fn create_vertex_array(&self) -> Arc<dyn VertexArray>;

    async fn create_texture(&self) -> Arc<dyn Texture>;
}

#[wasm_bus(format = "bincode")]
pub trait Buffer {
    async fn bind_buffer(&self, kind: BufferKind);
}

#[wasm_bus(format = "bincode")]
pub trait Texture {
    async fn active_texture(&self, active: u32);

    async fn bind_texture(&self, target: TextureKind);

    async fn bind_texture_cube(&self, target: TextureKind);

    async fn framebuffer_texture2d(&self, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32);
}

#[wasm_bus(format = "bincode")]
pub trait Raster {
    async fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32);
    
    async fn clear(&self, bit: BufferBit);

    async fn clear_depth(&self, value: f32);

    async fn draw_arrays(&self, mode: Primitives, first: i32, count: i32);

    async fn draw_elements(&self, mode: Primitives, count: i32, kind: DataType, offset: u32);

    async fn enable(&self, flag: Flag);

    async fn disable(&self, flag: Flag);

    async fn cull_face(&self, culling: Culling);

    async fn depth_mask(&self, val: bool);

    async fn depth_funct(&self, val: DepthTest);

    async fn viewport(&self, x: i32, y: i32, width: u32, height: u32);

    async fn buffer_data(&self, kind: BufferKind, data: Vec<u8>, draw: DrawMode);

    async fn unbind_buffer(&self, kind: BufferKind);

    async fn read_pixels(&self, x: u32, y: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType) -> Vec<u8>;

    async fn pixel_storei(&self, storage: PixelStorageMode, value: i32);

    async fn generate_mipmap(&self);

    async fn generate_mipmap_cube(&self);

    async fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    async fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u32, yoffset: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    async fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u32, height: u32, pixels: Vec<u8>);

    async fn unbind_texture(&self, active: u32);

    async fn unbind_texture_cube(&self, active: u32);

    async fn blend_equation(&self, eq: BlendEquation);

    async fn blend_func(&self, b1: BlendMode, b2: BlendMode);

    async fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32);

    async fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, param: i32);

    async fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, param: f32);

    async fn draw_buffers(&self, buffers: Vec<ColorBuffer>);

    async fn create_framebuffer(&self) -> Arc<dyn FrameBuffer>;

    async fn unbind_framebuffer(&self, buffer: Buffers);

    async fn unbind_vertex_array(&self);

    async fn sync(&self);
}

#[wasm_bus(format = "bincode")]
pub trait FrameBuffer {
    async fn bind_framebuffer(&self, buffer: Buffers);
}

#[wasm_bus(format = "bincode")]
pub trait Program {
    async fn create_shader(&self, kind: ShaderKind) -> Arc<dyn Shader>;

    async fn link_program(&self) -> Result<(), String>;

    async fn use_program(&self);

    async fn get_attrib_location(&self, name: String) -> Arc<dyn ProgramLocation>;

    async fn get_uniform_location(&self, name: String) -> Arc<dyn UniformLocation>;

    async fn get_program_parameter(&self, pname: ShaderParameter) -> Arc<dyn ProgramParameter>;
}

#[wasm_bus(format = "bincode")]
pub trait ProgramParameter {
}

#[wasm_bus(format = "bincode")]
pub trait ProgramLocation {
    async fn bind_program_location(&self);

    async fn vertex_attrib_pointer(&self, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32);

    async fn enable_vertex_attrib_array(&self);
}

#[wasm_bus(format = "bincode")]
pub trait VertexArray {
    async fn bind_vertex_array(&self);

    async fn unbind_vertex_array(&self);
}

#[wasm_bus(format = "bincode")]
pub trait UniformLocation {
    async fn uniform_matrix_4fv(&self, transpose: bool, value: [[f32; 4]; 4]);

    async fn uniform_matrix_3fv(&self, transpose: bool, value: [[f32; 3]; 3]);

    async fn uniform_matrix_2fv(&self, transpose: bool, value: [[f32; 2]; 2]);

    async fn uniform_1i(&self, value: i32);

    async fn uniform_1f(&self, value: f32);

    async fn uniform_2f(&self, value: (f32, f32));

    async fn uniform_3f(&self, value: (f32, f32, f32));

    async fn uniform_4f(&self, value: (f32, f32, f32, f32));
}

#[wasm_bus(format = "bincode")]
pub trait Shader {
    async fn shader_source(&self, source: String);

    async fn shader_compile(&self) -> Result<(), String>;

    async fn attach_shader(&self) -> Result<(), String>;
}