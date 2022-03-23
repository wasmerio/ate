use std::sync::Arc;
use async_trait::async_trait;
use wasm_bus_webgl::api::glenum::*;

#[async_trait]
pub trait WebGlAbi {
    async fn context(&self) -> Box<dyn RenderingContextAbi>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramId {
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId {
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexArrayId {
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureId {
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderId {
    pub program: ProgramId,
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramLocationId {
    pub program: ProgramId,
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniformLocationId {
    pub program: ProgramId,
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramParameterId {
    pub program: ProgramId,
    pub id: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameBufferId {
    pub id: u64
}

#[async_trait]
pub trait RenderingContextAbi {
    async fn create_program(&self) -> Option<ProgramId>;

    async fn create_buffer(&self) -> Option<BufferId>;

    async fn create_vertex_array(&self) -> Option<VertexArrayId>;

    async fn create_texture(&self) -> Option<TextureId>;

    async fn bind_buffer(&self, buffer: BufferId, kind: BufferKind);

    async fn delete_buffer(&self, buffer: BufferId);

    async fn delete_texture(&self, texture: TextureId);

    async fn active_texture(&self, texture: TextureId, active: u32);

    async fn bind_texture(&self, texture: TextureId);

    async fn bind_texture_cube(&self, texture: TextureId);

    async fn framebuffer_texture2d(&self, texture: TextureId, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32);

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

    async fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u16, height: u16, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    async fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u16, yoffset: u16, width: u16, height: u16, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    async fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u16, height: u16, data: Vec<u8>);

    async fn unbind_texture(&self);

    async fn unbind_texture_cube(&self);

    async fn blend_equation(&self, eq: BlendEquation);

    async fn blend_func(&self, b1: BlendMode, b2: BlendMode);

    async fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32);

    async fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, param: i32);

    async fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, param: f32);

    async fn draw_buffer(&self, buffers: Vec<ColorBuffer>);

    async fn create_framebuffer(&self) -> Option<FrameBufferId>;

    async fn unbind_framebuffer(&self, buffer: Buffers);

    async fn delete_framebuffer(&self, framebuffer: FrameBufferId);

    async fn bind_framebuffer(&self, framebuffer: FrameBufferId, buffer: Buffers);

    async fn delete_program(&self, program: ProgramId);

    async fn link_program(&self, program: ProgramId) -> Result<(), String>;

    async fn use_program(&self, program: ProgramId);

    async fn get_attrib_location(&self, program: ProgramId, name: String) -> Option<ProgramLocationId>;

    async fn get_uniform_location(&self, program: ProgramId, name: String) -> Option<UniformLocationId>;

    async fn get_program_parameter(&self, program: ProgramId, pname: ShaderParameter) -> Option<ProgramParameterId>;

    async fn delete_program_parameter(&self, param: ProgramParameterId);

    async fn delete_program_location(&self, location: ProgramLocationId);

    async fn bind_program_location(&self, location: ProgramLocationId) -> bool;

    async fn vertex_attrib_pointer(&self, location: ProgramLocationId, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32);

    async fn enable_vertex_attrib_array(&self, location: ProgramLocationId);
    
    async fn delete_vertex_array(&self, vertex_array: VertexArrayId);

    async fn bind_vertex_array(&self, vertex_array: VertexArrayId);

    async fn unbind_vertex_array(&self, vertex_array: VertexArrayId);

    async fn delete_uniform_location(&self, location: UniformLocationId);

    async fn uniform_matrix_4fv(&self, location: UniformLocationId, value: [[f32; 4]; 4]);

    async fn uniform_matrix_3fv(&self, location: UniformLocationId, value: [[f32; 3]; 3]);

    async fn uniform_matrix_2fv(&self, location: UniformLocationId, value: [[f32; 2]; 2]);

    async fn uniform_1i(&self, location: UniformLocationId, value: i32);

    async fn uniform_1f(&self, location: UniformLocationId, value: f32);

    async fn uniform_2f(&self, location: UniformLocationId, value: (f32, f32));

    async fn uniform_3f(&self, location: UniformLocationId, value: (f32, f32, f32));

    async fn uniform_4f(&self, location: UniformLocationId, value: (f32, f32, f32, f32));

    async fn create_shader(&self, program: ProgramId, kind: ShaderKind) -> Option<ShaderId>;

    async fn delete_shader(&self, shader: ShaderId);

    async fn shader_source(&self, shader: ShaderId, source: String);

    async fn shader_compile(&self, shader: ShaderId) -> Result<(), String>;

    async fn attach_shader(&self, shader: ShaderId) -> Result<(), String>;
}