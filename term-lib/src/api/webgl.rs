use wasm_bus_webgl::api::glenum::*;
pub use wasm_bus_webgl::api::glenum;

use crate::api::AsyncResult;

pub trait WebGlAbi
where Self: Send + Sync
{
    fn context(&self) -> Box<dyn RenderingContextAbi>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramId {
    pub id: u64
}

impl ProgramId {
    pub fn new() -> ProgramId {
        ProgramId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId {
    pub id: u64
}

impl BufferId {
    pub fn new() -> BufferId {
        BufferId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexArrayId {
    pub id: u64
}

impl VertexArrayId {
    pub fn new() -> VertexArrayId {
        VertexArrayId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureId {
    pub id: u64
}

impl TextureId {
    pub fn new() -> TextureId {
        TextureId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderId {
    pub id: u64
}

impl ShaderId {
    pub fn new() -> ShaderId {
        ShaderId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramLocationId {
    pub id: u64
}

impl ProgramLocationId {
    pub fn new() -> ProgramLocationId {
        ProgramLocationId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniformLocationId {
    pub id: u64
}

impl UniformLocationId {
    pub fn new() -> UniformLocationId {
        UniformLocationId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramParameterId {
    pub id: u64
}

impl ProgramParameterId {
    pub fn new() -> ProgramParameterId {
        ProgramParameterId {
            id: fastrand::u64(..)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameBufferId {
    pub id: u64
}

impl FrameBufferId {
    pub fn new() -> FrameBufferId {
        FrameBufferId {
            id: fastrand::u64(..)
        }
    }
}

pub trait RenderingContextAbi
where Self: Send + Sync
{
    fn create_program(&self) -> ProgramId;

    fn create_buffer(&self) -> BufferId;

    fn create_vertex_array(&self) -> VertexArrayId;

    fn create_texture(&self) -> TextureId;

    fn bind_buffer(&self, buffer: BufferId, kind: BufferKind);

    fn unbind_buffer(&self, kind: BufferKind);

    fn delete_buffer(&self, buffer: BufferId);

    fn delete_texture(&self, texture: TextureId);

    fn active_texture(&self, active: u32);

    fn bind_texture(&self, texture: TextureId, target: TextureKind);

    fn bind_texture_cube(&self, texture: TextureId, target: TextureKind);

    fn unbind_texture(&self, active: u32);

    fn unbind_texture_cube(&self, active: u32);

    fn framebuffer_texture2d(&self, texture: TextureId, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32);

    fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32);
    
    fn clear(&self, bit: BufferBit);

    fn clear_depth(&self, value: f32);

    fn draw_arrays(&self, mode: Primitives, first: i32, count: i32);

    fn draw_elements(&self, mode: Primitives, count: i32, kind: DataType, offset: u32);

    fn enable(&self, flag: Flag);

    fn disable(&self, flag: Flag);

    fn cull_face(&self, culling: Culling);

    fn depth_mask(&self, val: bool);

    fn depth_funct(&self, val: DepthTest);

    fn viewport(&self, x: i32, y: i32, width: u32, height: u32);

    fn buffer_data(&self, kind: BufferKind, data: Vec<u8>, draw: DrawMode);

    fn read_pixels(&self, x: u32, y: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType) -> AsyncResult<Result<Vec<u8>, String>>;

    fn pixel_storei(&self, storage: PixelStorageMode, value: i32);

    fn generate_mipmap(&self);

    fn generate_mipmap_cube(&self);

    fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u32, yoffset: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>);

    fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u32, height: u32, data: Vec<u8>);

    fn blend_equation(&self, eq: BlendEquation);

    fn blend_func(&self, b1: BlendMode, b2: BlendMode);

    fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32);

    fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, param: i32);

    fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, param: f32);

    fn draw_buffer(&self, buffers: Vec<ColorBuffer>);

    fn create_framebuffer(&self) -> FrameBufferId;

    fn delete_framebuffer(&self, framebuffer: FrameBufferId);

    fn bind_framebuffer(&self, framebuffer: FrameBufferId, buffer: Buffers);

    fn unbind_framebuffer(&self, buffer: Buffers);

    fn delete_program(&self, program: ProgramId);

    fn link_program(&self, program: ProgramId) -> AsyncResult<Result<(), String>>;

    fn use_program(&self, program: ProgramId);

    fn get_attrib_location(&self, program: ProgramId, name: String) -> ProgramLocationId;

    fn get_uniform_location(&self, program: ProgramId, name: String) -> UniformLocationId;

    fn get_program_parameter(&self, program: ProgramId, pname: ShaderParameter) -> ProgramParameterId;

    fn vertex_attrib_pointer(&self, location: ProgramLocationId, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32);

    fn delete_attrib_location(&self, location: ProgramLocationId);

    fn enable_vertex_attrib_array(&self, location: ProgramLocationId);
    
    fn delete_vertex_array(&self, vertex_array: VertexArrayId);

    fn bind_vertex_array(&self, vertex_array: VertexArrayId);

    fn unbind_vertex_array(&self);

    fn uniform_matrix_4fv(&self, location: UniformLocationId, transpose: bool, value: [[f32; 4]; 4]);

    fn uniform_matrix_3fv(&self, location: UniformLocationId, transpose: bool, value: [[f32; 3]; 3]);

    fn uniform_matrix_2fv(&self, location: UniformLocationId, transpose: bool, value: [[f32; 2]; 2]);

    fn uniform_1i(&self, location: UniformLocationId, value: i32);

    fn uniform_1f(&self, location: UniformLocationId, value: f32);

    fn uniform_2f(&self, location: UniformLocationId, value: (f32, f32));

    fn uniform_3f(&self, location: UniformLocationId, value: (f32, f32, f32));

    fn uniform_4f(&self, location: UniformLocationId, value: (f32, f32, f32, f32));

    fn create_shader(&self, kind: ShaderKind) -> ShaderId;

    fn delete_shader(&self, shader: ShaderId);

    fn shader_source(&self, shader: ShaderId, source: String);

    fn shader_compile(&self, shader: ShaderId) -> AsyncResult<Result<(), String>>;

    fn attach_shader(&self, program: ProgramId, shader: ShaderId) -> AsyncResult<Result<(), String>>;

    fn sync(&self) -> AsyncResult<()>;
}