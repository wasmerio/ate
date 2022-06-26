use wasm_bus_webgl::api::glenum::*;
use wasmer_vbus::BusDataFormat;
use std::sync::Arc;
use wasm_bus_webgl::api;
use std::ops::Deref;

use super::*;
use crate::api::*;

pub struct WebGlInstance {
    webgl: Box<dyn WebGlAbi>,
}

impl WebGlInstance {
    pub async fn new(system: System) -> Option<WebGlInstance> {
        let webgl = system.webgl().await?;
        Some(
            WebGlInstance {
                webgl,
            }
        )
    }

    pub fn context(&self) -> RenderingContextInstance {
        let ctx = self.webgl.context();
        RenderingContextInstance {
            ctx: Arc::new(ctx),
            std_ret_leaked: Arc::new(ResultInvokable::new_leaked(SerializationFormat::Bincode, ())),
        }
    }
}

impl Session
for WebGlInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::WebGlContextRequest>() => {
                let session = self.context();
                Ok((ResultInvokable::new_leaked(conv_format(format), ()), Some(Box::new(session))))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

#[derive(Clone)]
pub struct RenderingContextInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    std_ret_leaked: Arc<Box<ResultInvokable>>,
}

impl RenderingContextInstance {
    pub fn raster(&self) -> RasterInstance {
        let ctx = self.ctx.clone();
        RasterInstance {
            ctx,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
            std_ret_leaked: Arc::new(ResultInvokable::new_leaked(SerializationFormat::Bincode, ())),
        }
    }

    pub fn create_program(&self) -> ProgramInstance {
        let ctx = self.ctx.clone();
        let program = self.ctx.create_program();
        ProgramInstance {
            ctx,
            program,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
            std_ret_leaked: Arc::new(ResultInvokable::new_leaked(SerializationFormat::Bincode, ())),
        }
    }

    pub fn create_buffer(&self) -> BufferInstance {
        let ctx = self.ctx.clone();
        let buffer = self.ctx.create_buffer();
        BufferInstance {
            ctx,
            buffer,
        }
    }

    pub fn create_vertex_array(&self) -> VertexArrayInstance {
        let ctx = self.ctx.clone();
        let vertex_array = self.ctx.create_vertex_array();
        VertexArrayInstance {
            ctx,
            vertex_array,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
        }
    }

    pub fn create_texture(&self) -> TextureInstance {
        let ctx = self.ctx.clone();
        let texture = self.ctx.create_texture();
        TextureInstance {
            ctx,
            texture,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
        }
    }
}

impl Session
for RenderingContextInstance
{
    fn call(&mut self, topic_hash: u128, _format: BusDataFormat, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        let ret = self.std_ret_leaked.deref().clone();
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::RenderingContextRasterRequest>() => {
                let session = self.raster();
                Ok((ret, Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::RenderingContextCreateProgramRequest>() => {
                let session = self.create_program();
                Ok((ret, Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::RenderingContextCreateBufferRequest>() => {
                let session = self.create_buffer();
                Ok((ret, Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::RenderingContextCreateVertexArrayRequest>() => {
                let session = self.create_vertex_array();
                Ok((ret, Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::RenderingContextCreateTextureRequest>() => {
                let session = self.create_texture();
                Ok((ret, Some(Box::new(session))))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

pub struct BufferInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    buffer: BufferId,
}

impl BufferInstance
{
    pub fn bind_buffer(&self, kind: BufferKind) {
        self.ctx.bind_buffer(self.buffer, kind);
    }
}

impl Session
for BufferInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::BufferBindBufferRequest>() => {
                let request: api::BufferBindBufferRequest = decode_request(format, request)?;
                self.bind_buffer(request.kind);
                Ok((ResultInvokable::new_strong(conv_format(format), ()), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for BufferInstance
{
    fn drop(&mut self) {
        self.ctx.delete_buffer(self.buffer);
    }
}

pub struct TextureInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    texture: TextureId,
    std_ret: Arc<Box<ResultInvokable>>,
}

impl TextureInstance {
    pub fn bind_texture(&self, target: TextureKind) {
        self.ctx.bind_texture(self.texture, target);
    }

    pub fn bind_texture_cube(&self, target: TextureKind) {
        self.ctx.bind_texture_cube(self.texture, target);
    }

    pub fn framebuffer_texture2d(&self, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32) {
        self.ctx.framebuffer_texture2d(self.texture, target, attachment, textarget, level);
    }
}

impl Session
for TextureInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::TextureBindTextureRequest>() => {
                let request: api::TextureBindTextureRequest = decode_request(format, request)?;
                self.bind_texture(request.target);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::TextureBindTextureCubeRequest>() => {
                let request: api::TextureBindTextureCubeRequest = decode_request(format, request)?;
                self.bind_texture_cube(request.target);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::TextureFramebufferTexture2DRequest>() => {
                let request: api::TextureFramebufferTexture2DRequest = decode_request(format, request)?;
                self.framebuffer_texture2d(request.target, request.attachment, request.textarget, request.level);
                Ok((self.std_ret.deref().clone(), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for TextureInstance
{
    fn drop(&mut self) {
        self.ctx.delete_texture(self.texture);
    }
}

pub struct RasterInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    std_ret: Arc<Box<ResultInvokable>>,
    std_ret_leaked: Arc<Box<ResultInvokable>>,
}

impl RasterInstance
{
    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.ctx.clear_color(red, green, blue, alpha);
    }
    
    pub fn clear(&self, bit: BufferBit) {
        self.ctx.clear(bit);
    }

    pub fn clear_depth(&self, value: f32) {
        self.ctx.clear_depth(value);
    }

    pub fn draw_arrays(&self, mode: Primitives, first: i32, count: i32) {
        self.ctx.draw_arrays(mode, first, count);
    }

    pub fn draw_elements(&self, mode: Primitives, count: i32, kind: DataType, offset: u32) {
        self.ctx.draw_elements(mode, count, kind, offset);
    }

    pub fn enable(&self, flag: Flag) {
        self.ctx.enable(flag);
    }

    pub fn disable(&self, flag: Flag) {
        self.ctx.disable(flag);
    }

    pub fn cull_face(&self, culling: Culling) {
        self.ctx.cull_face(culling);
    }

    pub fn depth_mask(&self, val: bool) {
        self.ctx.depth_mask(val);
    }

    pub fn depth_funct(&self, val: DepthTest) {
        self.ctx.depth_funct(val);
    }

    pub fn viewport(&self, x: i32, y: i32, width: u32, height: u32) {
        self.ctx.viewport(x, y, width, height);
    }

    pub fn buffer_data(&self, kind: BufferKind, data: Vec<u8>, draw: DrawMode) {
        self.ctx.buffer_data(kind, data, draw);
    }    

    pub fn read_pixels(&self, x: u32, y: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType) -> AsyncResult<Result<Vec<u8>, String>> {
        self.ctx.read_pixels(x, y, width, height, format, kind, SerializationFormat::Bincode)
    }

    pub fn pixel_storei(&self, storage: PixelStorageMode, value: i32) {
        self.ctx.pixel_storei(storage, value);
    }

    pub fn generate_mipmap(&self) {
        self.ctx.generate_mipmap();
    }

    pub fn generate_mipmap_cube(&self) {
        self.ctx.generate_mipmap_cube();
    }

    pub fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>) {
        self.ctx.tex_image2d(target, level, width, height, format, kind, pixels);
    }

    pub fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u32, yoffset: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType, pixels: Vec<u8>) {
        self.ctx.tex_sub_image2d(target, level, xoffset, yoffset, width, height, format, kind, pixels);
    }

    pub fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u32, height: u32, pixels: Vec<u8>) {
        self.ctx.compressed_tex_image2d(target, level, compression, width, height, pixels);
    }

    pub fn active_texture(&self, active: u32) {
        self.ctx.active_texture(active);
    }

    pub fn blend_equation(&self, eq: BlendEquation) {
        self.ctx.blend_equation(eq);
    }

    pub fn blend_func(&self, b1: BlendMode, b2: BlendMode) {
        self.ctx.blend_func(b1, b2);
    }

    pub fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.ctx.blend_color(red, green, blue, alpha);
    }

    pub fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, param: i32) {
        self.ctx.tex_parameteri(kind, pname, param);
    }

    pub fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, param: f32) {
        self.ctx.tex_parameterfv(kind, pname, param);
    }

    pub fn draw_buffers(&self, buffers: Vec<ColorBuffer>) {
        self.ctx.draw_buffers(buffers);
    }

    pub fn create_framebuffer(&self) -> FrameBufferInstance {
        let framebuffer = self.ctx.create_framebuffer();
        FrameBufferInstance {
            ctx: self.ctx.clone(),
            framebuffer,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
        }
    }

    pub fn unbind_buffer(&self, kind: BufferKind) {
        self.ctx.unbind_buffer(kind);
    }

    pub fn unbind_texture(&self, active: u32) {
        self.ctx.unbind_texture(active);
    }

    pub fn unbind_texture_cube(&self, active: u32) {
        self.ctx.unbind_texture_cube(active);
    }

    pub fn unbind_vertex_array(&self) {
        self.ctx.unbind_vertex_array();
    }

    pub fn unbind_framebuffer(&self, buffer: Buffers) {
        self.ctx.unbind_framebuffer(buffer);
    }

    pub fn sync(&self) -> AsyncResult<()> {
        self.ctx.sync(SerializationFormat::Bincode)
    }
}

impl Session
for RasterInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::RasterClearColorRequest>() => {
                let request: api::RasterClearColorRequest = decode_request(format, request)?;
                self.clear_color(request.red, request.green, request.blue, request.alpha);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterClearRequest>() => {
                let request: api::RasterClearRequest = decode_request(format, request)?;
                self.clear(request.bit);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterClearDepthRequest>() => {
                let request: api::RasterClearDepthRequest = decode_request(format, request)?;
                self.clear_depth(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDrawArraysRequest>() => {
                let request: api::RasterDrawArraysRequest = decode_request(format, request)?;
                self.draw_arrays(request.mode, request.first, request.count);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDrawElementsRequest>() => {
                let request: api::RasterDrawElementsRequest = decode_request(format, request)?;
                self.draw_elements(request.mode, request.count, request.kind, request.offset);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterEnableRequest>() => {
                let request: api::RasterEnableRequest = decode_request(format, request)?;
                self.enable(request.flag);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDisableRequest>() => {
                let request: api::RasterDisableRequest = decode_request(format, request)?;
                self.disable(request.flag);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterCullFaceRequest>() => {
                let request: api::RasterCullFaceRequest = decode_request(format, request)?;
                self.cull_face(request.culling);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDepthMaskRequest>() => {
                let request: api::RasterDepthMaskRequest = decode_request(format, request)?;
                self.depth_mask(request.val);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDepthFunctRequest>() => {
                let request: api::RasterDepthFunctRequest = decode_request(format, request)?;
                self.depth_funct(request.val);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterViewportRequest>() => {
                let request: api::RasterViewportRequest = decode_request(format, request)?;
                self.viewport(request.x, request.y, request.width, request.height);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterBufferDataRequest>() => {
                let request: api::RasterBufferDataRequest = decode_request(format, request)?;
                self.buffer_data(request.kind, request.data, request.draw);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterReadPixelsRequest>() => {
                let request: api::RasterReadPixelsRequest = decode_request(format, request)?;
                let ret = self.read_pixels(request.x, request.y, request.width, request.height, request.format, request.kind);
                Ok((Box::new(ret), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterPixelStoreiRequest>() => {
                let request: api::RasterPixelStoreiRequest = decode_request(format, request)?;
                self.pixel_storei(request.storage, request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterGenerateMipmapRequest>() => {
                self.generate_mipmap();
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterGenerateMipmapCubeRequest>() => {
                self.generate_mipmap_cube();
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterTexImage2DRequest>() => {
                let request: api::RasterTexImage2DRequest = decode_request(format, request)?;
                self.tex_image2d(request.target, request.level, request.width, request.height, request.format, request.kind, request.pixels);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterTexSubImage2DRequest>() => {
                let request: api::RasterTexSubImage2DRequest = decode_request(format, request)?;
                self.tex_sub_image2d(request.target, request.level, request.xoffset, request.yoffset, request.width, request.height, request.format, request.kind, request.pixels);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterCompressedTexImage2DRequest>() => {
                let request: api::RasterCompressedTexImage2DRequest = decode_request(format, request)?;
                self.compressed_tex_image2d(request.target, request.level, request.compression, request.width, request.height, request.pixels);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::TextureActiveTextureRequest>() => {
                let request: api::TextureActiveTextureRequest = decode_request(format, request)?;
                self.active_texture(request.active);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterBlendEquationRequest>() => {
                let request: api::RasterBlendEquationRequest = decode_request(format, request)?;
                self.blend_equation(request.eq);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterBlendEquationRequest>() => {
                let request: api::RasterBlendEquationRequest = decode_request(format, request)?;
                self.blend_equation(request.eq);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterBlendFuncRequest>() => {
                let request: api::RasterBlendFuncRequest = decode_request(format, request)?;
                self.blend_func(request.b1, request.b2);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterBlendColorRequest>() => {
                let request: api::RasterBlendColorRequest = decode_request(format, request)?;
                self.blend_color(request.red, request.green, request.blue, request.alpha);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterTexParameteriRequest>() => {
                let request: api::RasterTexParameteriRequest = decode_request(format, request)?;
                self.tex_parameteri(request.kind, request.pname, request.param);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterTexParameterfvRequest>() => {
                let request: api::RasterTexParameterfvRequest = decode_request(format, request)?;
                self.tex_parameterfv(request.kind, request.pname, request.param);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterDrawBuffersRequest>() => {
                let request: api::RasterDrawBuffersRequest = decode_request(format, request)?;
                self.draw_buffers(request.buffers);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterCreateFramebufferRequest>() => {
                let session = self.create_framebuffer();
                Ok((self.std_ret_leaked.deref().clone(), Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterUnbindBufferRequest>() => {
                let request: api::RasterUnbindBufferRequest = decode_request(format, request)?;
                self.unbind_buffer(request.kind);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterUnbindTextureRequest>() => {
                let request: api::RasterUnbindTextureRequest = decode_request(format, request)?;
                self.unbind_texture(request.active);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterUnbindTextureCubeRequest>() => {
                let request: api::RasterUnbindTextureCubeRequest = decode_request(format, request)?;
                self.unbind_texture_cube(request.active);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterUnbindVertexArrayRequest>() => {
                self.unbind_vertex_array();
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterUnbindFramebufferRequest>() => {
                let request: api::RasterUnbindFramebufferRequest = decode_request(format, request)?;
                self.unbind_framebuffer(request.buffer);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::RasterSyncRequest>() => {
                let ret = self.sync();
                Ok((Box::new(ret), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

pub struct FrameBufferInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    framebuffer: FrameBufferId,
    std_ret: Arc<Box<ResultInvokable>>,
}

impl FrameBufferInstance {
    pub fn bind_framebuffer(&self, buffer: Buffers) {
        self.ctx.bind_framebuffer(self.framebuffer, buffer);
    }
}

impl Session
for FrameBufferInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::FrameBufferBindFramebufferRequest>() => {
                let request: api::FrameBufferBindFramebufferRequest = decode_request(format, request)?;
                self.bind_framebuffer(request.buffer);
                Ok((self.std_ret.deref().clone(), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for FrameBufferInstance
{
    fn drop(&mut self) {
        self.ctx.delete_framebuffer(self.framebuffer);
    }
}

pub struct ProgramInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    program: ProgramId,
    std_ret: Arc<Box<ResultInvokable>>,
    std_ret_leaked: Arc<Box<ResultInvokable>>,
}

impl ProgramInstance {
    pub fn create_shader(&self, kind: ShaderKind) -> ShaderInstance {
        let shader = self.ctx.create_shader(kind);
        ShaderInstance {
            ctx: self.ctx.clone(),
            program: self.program.clone(),
            shader,
        }
    }

    pub fn link_program(&self) -> AsyncResult<Result<(), String>> {
        self.ctx.link_program(self.program, SerializationFormat::Bincode)
    }

    pub fn use_program(&self) {
        self.ctx.use_program(self.program);
    }

    pub fn get_attrib_location(&self, name: String) -> ProgramLocationInstance {
        let location = self.ctx.get_attrib_location(self.program, name);
        ProgramLocationInstance {
            ctx: self.ctx.clone(),
            location,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
        }
    }

    pub fn get_uniform_location(&self, name: String) -> UniformLocationInstance {
        let location = self.ctx.get_uniform_location(self.program, name);
        UniformLocationInstance {
            ctx: self.ctx.clone(),
            location,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
        }
    }

    pub fn get_program_parameter(&self, pname: ShaderParameter) -> ProgramParameterInstance {
        let param = self.ctx.get_program_parameter(self.program, pname);
        ProgramParameterInstance {
            ctx: self.ctx.clone(),
            param,
            std_ret: Arc::new(ResultInvokable::new_strong(SerializationFormat::Bincode, ())),
            std_ret_leaked: Arc::new(ResultInvokable::new_leaked(SerializationFormat::Bincode, ())),
        }
    }
}

impl Session
for ProgramInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::ProgramCreateShaderRequest>() => {
                let request: api::ProgramCreateShaderRequest = decode_request(format, request)?;
                let session = self.create_shader(request.kind);
                Ok((self.std_ret_leaked.deref().clone(), Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramLinkProgramRequest>() => {
                let ret = self.link_program();
                Ok((Box::new(ret), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramUseProgramRequest>() => {
                self.use_program();
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramGetAttribLocationRequest>() => {
                let request: api::ProgramGetAttribLocationRequest = decode_request(format, request)?;
                let session = self.get_attrib_location(request.name);
                Ok((self.std_ret_leaked.deref().clone(), Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramGetUniformLocationRequest>() => {
                let request: api::ProgramGetUniformLocationRequest = decode_request(format, request)?;
                let session = self.get_uniform_location(request.name);
                Ok((self.std_ret_leaked.deref().clone(), Some(Box::new(session))))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramGetProgramParameterRequest>() => {
                let request: api::ProgramGetProgramParameterRequest = decode_request(format, request)?;
                let session = self.get_program_parameter(request.pname);
                Ok((self.std_ret_leaked.deref().clone(), Some(Box::new(session))))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for ProgramInstance
{
    fn drop(&mut self) {
        self.ctx.delete_program(self.program);
    }
}

#[allow(dead_code)]
pub struct ProgramParameterInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    param: ProgramParameterId,
    std_ret: Arc<Box<ResultInvokable>>,
    std_ret_leaked: Arc<Box<ResultInvokable>>,
}

impl ProgramParameterInstance {
}

impl Session
for ProgramParameterInstance
{
    fn call(&mut self, _topic_hash: u128, _format: BusDataFormat, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        Err(BusError::InvalidTopic)
    }
}

pub struct ProgramLocationInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    location: ProgramLocationId,
    std_ret: Arc<Box<ResultInvokable>>,
}

impl ProgramLocationInstance {
    pub fn vertex_attrib_pointer(&self, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32) {
        self.ctx.vertex_attrib_pointer(self.location, size, kind, normalized, stride, offset);
    }

    pub fn enable_vertex_attrib_array(&self) {
        self.ctx.enable_vertex_attrib_array(self.location);
    }
}

impl Drop
for ProgramLocationInstance
{
    fn drop(&mut self) {
        self.ctx.delete_attrib_location(self.location);
    }
}

impl Session
for ProgramLocationInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::ProgramLocationVertexAttribPointerRequest>() => {
                let request: api::ProgramLocationVertexAttribPointerRequest = decode_request(format, request)?;
                self.vertex_attrib_pointer(request.size, request.kind, request.normalized, request.stride, request.offset);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::ProgramLocationEnableVertexAttribArrayRequest>() => {
                self.enable_vertex_attrib_array();
                Ok((self.std_ret.deref().clone(), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

pub struct VertexArrayInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    vertex_array: VertexArrayId,
    std_ret: Arc<Box<ResultInvokable>>,
}

impl VertexArrayInstance {
    pub fn bind_vertex_array(&self) {
        self.ctx.bind_vertex_array(self.vertex_array);
    }
}

impl Session
for VertexArrayInstance
{
    fn call(&mut self, topic_hash: u128, _format: BusDataFormat, _request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::VertexArrayBindVertexArrayRequest>() => {
                self.bind_vertex_array();
                Ok((self.std_ret.deref().clone(), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for VertexArrayInstance
{
    fn drop(&mut self) {
        self.ctx.delete_vertex_array(self.vertex_array);
    }
}

#[derive(Clone)]
pub struct UniformLocationInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    location: UniformLocationId,
    std_ret: Arc<Box<ResultInvokable>>,
}

impl UniformLocationInstance {
    pub fn uniform_matrix_4fv(&self, transpose: bool, value: [[f32; 4]; 4]) {
        self.ctx.uniform_matrix_4fv(self.location, transpose, value);
    }

    pub fn uniform_matrix_3fv(&self, transpose: bool, value: [[f32; 3]; 3]) {
        self.ctx.uniform_matrix_3fv(self.location, transpose, value);
    }

    pub fn uniform_matrix_2fv(&self, transpose: bool, value: [[f32; 2]; 2]) {
        self.ctx.uniform_matrix_2fv(self.location, transpose, value);
    }

    pub fn uniform_1i(&self, value: i32) {
        self.ctx.uniform_1i(self.location, value);
    }

    pub fn uniform_1f(&self, value: f32) {
        self.ctx.uniform_1f(self.location, value);
    }

    pub fn uniform_2f(&self, value: (f32, f32)) {
        self.ctx.uniform_2f(self.location, value);
    }

    pub fn uniform_3f(&self, value: (f32, f32, f32)) {
        self.ctx.uniform_3f(self.location, value);
    }

    pub fn uniform_4f(&self, value: (f32, f32, f32, f32)) {
        self.ctx.uniform_4f(self.location, value);
    }
}

impl Session
for UniformLocationInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniformMatrix4FvRequest>() => {
                let request: api::UniformLocationUniformMatrix4FvRequest = decode_request(format, request)?;
                self.uniform_matrix_4fv(request.transpose, request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniformMatrix3FvRequest>() => {
                let request: api::UniformLocationUniformMatrix3FvRequest = decode_request(format, request)?;
                self.uniform_matrix_3fv(request.transpose, request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniformMatrix2FvRequest>() => {
                let request: api::UniformLocationUniformMatrix2FvRequest = decode_request(format, request)?;
                self.uniform_matrix_2fv(request.transpose, request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniform1IRequest>() => {
                let request: api::UniformLocationUniform1IRequest = decode_request(format, request)?;
                self.uniform_1i(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniform1FRequest>() => {
                let request: api::UniformLocationUniform1FRequest = decode_request(format, request)?;
                self.uniform_1f(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniform2FRequest>() => {
                let request: api::UniformLocationUniform2FRequest = decode_request(format, request)?;
                self.uniform_2f(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniform3FRequest>() => {
                let request: api::UniformLocationUniform3FRequest = decode_request(format, request)?;
                self.uniform_3f(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::UniformLocationUniform4FRequest>() => {
                let request: api::UniformLocationUniform4FRequest = decode_request(format, request)?;
                self.uniform_4f(request.value);
                Ok((self.std_ret.deref().clone(), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

pub struct ShaderInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    program: ProgramId,
    shader: ShaderId,
}

impl ShaderInstance {
    pub fn shader_source(&self, source: String) {
        self.ctx.shader_source(self.shader, source);
    }

    pub fn shader_compile(&self) -> AsyncResult<Result<(), String>> {
        self.ctx.shader_compile(self.shader, SerializationFormat::Bincode)
    }

    pub fn attach_shader(&self) -> AsyncResult<Result<(), String>> {
        self.ctx.attach_shader(self.program, self.shader, SerializationFormat::Bincode)
    }
}

impl Session
for ShaderInstance
{
    fn call(&mut self, topic_hash: u128, format: BusDataFormat, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Processable + 'static>, Option<Box<dyn Session + 'static>>), BusError> {
        match topic_hash {
            topic_hash if topic_hash == type_name_hash::<api::ShaderShaderSourceRequest>() => {
                let request: api::ShaderShaderSourceRequest = decode_request(format, request)?;
                self.shader_source(request.source);
                Ok((ResultInvokable::new(conv_format(format), ()), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::ShaderShaderCompileRequest>() => {
                let ret = self.shader_compile();
                Ok((Box::new(ret), None))
            }
            topic_hash if topic_hash == type_name_hash::<api::ShaderAttachShaderRequest>() => {
                let ret = self.attach_shader();
                Ok((Box::new(ret), None))
            }
            _ => Err(BusError::InvalidTopic)
        }
    }
}

impl Drop
for ShaderInstance
{
    fn drop(&mut self) {
        self.ctx.delete_shader(self.shader);
    }
}