use wasm_bus_webgl::prelude::*;
use wasm_bus_webgl::api::glenum::*;
use tokio::sync::Mutex;
use std::sync::Arc;
use wasm_bus_webgl::api;

use super::*;
use crate::api::*;

pub struct WebGlInstance {
    abi: Arc<dyn SystemAbi>,
    webgl: Box<dyn WebGlAbi>,
}

impl WebGlInstance {
    pub async fn new(abi: &Arc<dyn SystemAbi>) -> WebGlInstance {
        let webgl = abi.webgl().await;
        WebGlInstance {
            abi: abi.clone(),
            webgl,
        }
    }

    pub async fn context(&self) -> RenderingContextInstance {
        let ctx = self.webgl.context().await;
        RenderingContextInstance {
            ctx: Arc::new(ctx)
        }
    }
}

impl Session
for WebGlInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::WebGlContextRequest>() => {
                let _request: api::WebGlContextRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.context().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

pub struct RenderingContextInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
}

impl RenderingContextInstance {
    pub async fn raster(&self) -> Result<RasterInstance, CallError> {
        let ctx = self.ctx.clone();
        Ok(
            RasterInstance {
                ctx,
            }
        )
    }

    pub async fn create_program(&self) -> Result<ProgramInstance, CallError> {
        let ctx = self.ctx.clone();
        let program = self.ctx.create_program().await
            .ok_or(CallError::Unsupported)?;
        Ok(
            ProgramInstance {
                ctx,
                program
            }
        )
    }

    pub async fn create_buffer(&self) -> Result<BufferInstance, CallError> {
        let ctx = self.ctx.clone();
        let buffer = self.ctx.create_buffer().await
            .ok_or(CallError::Unsupported)?;
        Ok(
            BufferInstance {
                ctx,
                buffer,
            }
        )
    }

    pub async fn create_vertex_array(&self) -> Result<VertexArrayInstance, CallError> {
        let ctx = self.ctx.clone();
        let vertex_array = self.ctx.create_vertex_array().await
            .ok_or(CallError::Unsupported)?;
        Ok(
            VertexArrayInstance {
                ctx,
                vertex_array,
            }
        )
    }

    pub async fn create_texture(&self) -> Result<TextureInstance, CallError> {
        let ctx = self.ctx.clone();
        let texture = self.ctx.create_texture().await
            .ok_or(CallError::Unsupported)?;
        Ok(
            TextureInstance {
                ctx,
                texture,
            }
        )
    }
}

impl Session
for RenderingContextInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::RenderingContextRasterRequest>() => {
                let _request: api::RenderingContextRasterRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.raster().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            topic if topic == type_name::<api::RenderingContextCreateProgramRequest>() => {
                let _request: api::RenderingContextCreateProgramRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.create_program().await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            topic if topic == type_name::<api::RenderingContextCreateBufferRequest>() => {
                let _request: api::RenderingContextCreateBufferRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.create_buffer().await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            topic if topic == type_name::<api::RenderingContextCreateVertexArrayRequest>() => {
                let _request: api::RenderingContextCreateVertexArrayRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.create_vertex_array().await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            topic if topic == type_name::<api::RenderingContextCreateTextureRequest>() => {
                let _request: api::RenderingContextCreateTextureRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.create_texture().await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

pub struct BufferInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    buffer: BufferId,
}

impl BufferInstance
{
    pub async fn bind_buffer(&self, kind: BufferKind) {
        self.ctx.bind_buffer(self.buffer, kind).await;
    }
}

impl Session
for BufferInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::BufferBindBufferRequest>() => {
                let request: api::BufferBindBufferRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.bind_buffer(request.kind).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for BufferInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let buffer = self.buffer.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_buffer(buffer).await;
        });
    }
}

pub struct TextureInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    texture: TextureId,
}

impl TextureInstance {
    pub async fn active_texture(&self, active: u32) {
        self.ctx.active_texture(self.texture, active).await;
    }

    pub async fn bind_texture(&self) {
        self.ctx.bind_texture(self.texture).await;
    }

    pub async fn bind_texture_cube(&self) {
        self.ctx.bind_texture_cube(self.texture).await;
    }

    pub async fn framebuffer_texture2d(&self, target: Buffers, attachment: Buffers, textarget: TextureBindPoint, level: i32) {
        self.ctx.framebuffer_texture2d(self.texture, target, attachment, textarget, level).await;
    }
}

impl Session
for TextureInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::TextureActiveTextureRequest>() => {
                let request: api::TextureActiveTextureRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.active_texture(request.active).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::TextureBindTextureRequest>() => {
                let _request: api::TextureBindTextureRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.bind_texture().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::TextureBindTextureCubeRequest>() => {
                let _request: api::TextureBindTextureCubeRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.bind_texture_cube().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::TextureFramebufferTexture2DRequest>() => {
                let request: api::TextureFramebufferTexture2DRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.framebuffer_texture2d(request.target, request.attachment, request.textarget, request.level).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for TextureInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let texture = self.texture.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_texture(buffer).await;
        });
    }
}

pub struct RasterInstance {
    ctx: Arc<dyn RenderingContextAbi>,
}

#[async_trait]
impl RasterInstance
{
    pub async fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.ctx.clear_color(red, green, blue, alpha).await;
    }
    
    pub async fn clear(&self, bit: BufferBit) {
        self.ctx.clear(bit).await;
    }

    pub async fn clear_depth(&self, value: f32) {
        self.ctx.clear_depth(value).await;
    }

    pub async fn draw_arrays(&self, mode: Primitives, first: i32, count: i32) {
        self.ctx.draw_arrays(mode, first, count).await;
    }

    pub async fn draw_elements(&self, mode: Primitives, count: i32, kind: DataType, offset: u32) {
        self.ctx.draw_elements(mode, count, kind, offset).await;
    }

    pub async fn enable(&self, flag: Flag) {
        self.ctx.enable(flag).await;
    }

    pub async fn disable(&self, flag: Flag) {
        self.ctx.disable(flag).await;
    }

    pub async fn cull_face(&self, culling: Culling) {
        self.ctx.cull_face(culling).await;
    }

    pub async fn depth_mask(&self, val: bool) {
        self.ctx.depth_mask(val).await;
    }

    pub async fn depth_funct(&self, val: DepthTest) {
        self.ctx.depth_funct(val).await;
    }

    pub async fn viewport(&self, x: i32, y: i32, width: u32, height: u32) {
        self.ctx.viewport(x, y, width, height).await;
    }

    pub async fn buffer_data(&self, kind: BufferKind, data: Vec<u8>, draw: DrawMode) {
        self.ctx.buffer_data(kind, data, draw).await;
    }

    pub async fn unbind_buffer(&self, kind: BufferKind) {
        self.ctx.unbind_buffer(kind).await;
    }

    pub async fn read_pixels(&self, x: u32, y: u32, width: u32, height: u32, format: PixelFormat, kind: PixelType) -> Vec<u8> {
        self.ctx.read_pixels(x, y, width, height, format, kind).await
    }

    pub async fn pixel_storei(&self, storage: PixelStorageMode, value: i32) {
        self.ctx.pixel_storei(storage, value).await;
    }

    pub async fn generate_mipmap(&self) {
        self.ctx.generate_mipmap().await;
    }

    pub async fn generate_mipmap_cube(&self) {
        self.ctx.generate_mipmap_cube().await;
    }

    pub async fn tex_image2d(&self, target: TextureBindPoint, level: u8, width: u16, height: u16, format: PixelFormat, kind: PixelType, pixels: Vec<u8>) {
        self.ctx.tex_image2d(target, level, width, height, format, kind, pixels).await;
    }

    pub async fn tex_sub_image2d(&self, target: TextureBindPoint, level: u8, xoffset: u16, yoffset: u16, width: u16, height: u16, format: PixelFormat, kind: PixelType, pixels: Vec<u8>) {
        self.ctx.text_sub_image2d(target, level, xoffset, yoffset, width, height, format, kind, pixels).await;
    }

    pub async fn compressed_tex_image2d(&self, target: TextureBindPoint, level: u8, compression: TextureCompression, width: u16, height: u16, data: Vec<u8>) {
        self.ctx.compressed_text_image2d(target, level, compression, width, height, data).await;
    }

    pub async fn unbind_texture(&self) {
        self.ctx.unbind_texture().await;
    }

    pub async fn unbind_texture_cube(&self) {
        self.ctx.unbind_texture_cube().await;
    }

    pub async fn blend_equation(&self, eq: BlendEquation) {
        self.ctx.blend_equation(eq).await;
    }

    pub async fn blend_func(&self, b1: BlendMode, b2: BlendMode) {
        self.ctx.blend_func(b1, b2).await;
    }

    pub async fn blend_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        self.ctx.blend_color(red, green, blue, alpha).await;
    }

    pub async fn tex_parameteri(&self, kind: TextureKind, pname: TextureParameter, param: i32) {
        self.ctx.tex_parameteri(kind, pname, param).await;
    }

    pub async fn tex_parameterfv(&self, kind: TextureKind, pname: TextureParameter, param: f32) {
        self.ctx.tex_parameterfv(kind, pname, param).await;
    }

    pub async fn draw_buffer(&self, buffers: Vec<ColorBuffer>) {
        self.ctx.draw_buffer(buffers).await;
    }

    pub async fn create_framebuffer(&self) -> Result<FrameBufferInstance, CallError> {
        let framebuffer = self.ctx.create_framebuffer().await
            .ok_or(CallError::Unsupported)?;
        Ok(
            FrameBufferInstance {
                ctx: self.ctx.clone(),
                framebuffer
            }
        )
    }

    pub async fn unbind_framebuffer(&self, buffer: Buffers) {
        self.ctx.unbind_framebuffer(buffer).await;
    }
}

impl Session
for RasterInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::RasterClearColorRequest>() => {
                let request: api::RasterClearColorRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.clear_color(request.red, request.green, request.blue, request.alpha).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterClearRequest>() => {
                let request: api::RasterClearRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.clear(request.bit).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterClearDepthRequest>() => {
                let request: api::RasterClearDepthRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.clear_depth(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDrawArraysRequest>() => {
                let request: api::RasterDrawArraysRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.draw_arrays(request.mode, request.first, request.count).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDrawElementsRequest>() => {
                let request: api::RasterDrawElementsRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.draw_elements(request.mode, request.count, request.kind, request.offset).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterEnableRequest>() => {
                let request: api::RasterEnableRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.enable(request.flag).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDisableRequest>() => {
                let request: api::RasterDisableRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.disable(request.flag).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterCullFaceRequest>() => {
                let request: api::RasterCullFaceRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.cull_face(request.culling).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDepthMaskRequest>() => {
                let request: api::RasterDepthMaskRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.depth_mask(request.val).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDepthFunctRequest>() => {
                let request: api::RasterDepthFunctRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.depth_funct(request.val).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterViewportRequest>() => {
                let request: api::RasterViewportRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.viewport(request.x, request.y, request.width, request.height).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterBufferDataRequest>() => {
                let request: api::RasterBufferDataRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.buffer_data(request.kind, request.data, request.draw).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterUnbindBufferRequest>() => {
                let request: api::RasterUnbindBufferRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.unbind_buffer(request.kind).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterReadPixelsRequest>() => {
                let request: api::RasterReadPixelsRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.read_pixels(request.x, request.y, request.width, request.height, request.format, request.kind).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterPixelStoreiRequest>() => {
                let request: api::RasterPixelStoreiRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.pixel_storei(request.storage, request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterGenerateMipmapRequest>() => {
                let _request: api::RasterGenerateMipmapRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.generate_mipmap().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterGenerateMipmapCubeRequest>() => {
                let request: api::RasterGenerateMipmapCubeRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.generate_mipmap_cube().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterTexImage2DRequest>() => {
                let request: api::RasterTexImage2DRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.tex_image2d(request.target, request.level, request.width, request.height, request.format, request.kind, request.pixels).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterTexSubImage2DRequest>() => {
                let request: api::RasterTexSubImage2DRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.tex_sub_image2d(request.target, request.level, request.xoffset, request.yoffset, request.width, request.height, request.format, request.kind, request.pixels).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterCompressedTexImage2DRequest>() => {
                let request: api::RasterCompressedTexImage2DRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.compressed_tex_image2d(request.target, request.level, request.compression, request.width, request.height, request.data).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterUnbindTextureRequest>() => {
                let _request: api::RasterUnbindTextureRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.unbind_texture().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterUnbindTextureCubeRequest>() => {
                let _request: api::RasterUnbindTextureCubeRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.unbind_texture_cube().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterBlendEquationRequest>() => {
                let request: api::RasterBlendEquationRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.blend_equation(request.eq).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterBlendEquationRequest>() => {
                let request: api::RasterBlendEquationRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.blend_equation(request.eq).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterBlendFuncRequest>() => {
                let request: api::RasterBlendFuncRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.blend_func(request.b1, request.b2).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterBlendColorRequest>() => {
                let request: api::RasterBlendColorRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.blend_color(request.red, request.green, request.blue, request.alpha).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterTexParameteriRequest>() => {
                let request: api::RasterTexParameteriRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.tex_parameteri(request.kind, request.pname, request.param).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterTexParameterfvRequest>() => {
                let request: api::RasterTexParameterfvRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.tex_parameterfv(request.kind, request.pname, request.param).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterDrawBufferRequest>() => {
                let request: api::RasterDrawBufferRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.draw_buffer(request.buffers).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterCreateFramebufferRequest>() => {
                let _request: api::RasterCreateFramebufferRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.create_framebuffer().await?;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::RasterUnbindFramebufferRequest>() => {
                let request: api::RasterUnbindFramebufferRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let session = self.unbind_framebuffer(request.buffer).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ()), Some(Box::new(session))))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

pub struct FrameBufferInstance {
    ctx: Arc<Box<dyn RenderingContextAbi>>,
    framebuffer: FrameBufferId,
}

impl FrameBufferInstance {
    pub async fn bind_framebuffer(&self, buffer: Buffers) {
        self.ctx.bind_framebuffer(self.framebuffer, buffer).await;
    }
}

impl Session
for FrameBufferInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::FrameBufferBindFramebufferRequest>() => {
                let request: api::FrameBufferBindFramebufferRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.bind_framebuffer(request.buffer).await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for FrameBufferInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let framebuffer = self.framebuffer.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_framebuffer(framebuffer).await;
        });
    }
}

pub struct ProgramInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    program: ProgramId,
}

impl ProgramInstance {
    pub async fn create_shader(&self, kind: ShaderKind) -> Result<ShaderInstance, CallError> {
        let shader = self.ctx.create_shader(kind).await
            .ok_or(CallError::Unsupported)?;
        Ok(
            ShaderInstance {
                ctx: self.ctx.clone(),
                shader,
            }
        )
    }

    pub async fn link_program(&self) -> Result<(), String> {
        self.ctx.link_program(self.program).await
    }

    pub async fn use_program(&self) {
        self.ctx.use_program(self.program).await;
    }

    pub async fn get_attrib_location(&self, name: String) -> Result<ProgramLocationInstance, CallError> {
        let location = self.ctx.get_attrib_location(name).await
            .ok_or(CallError::Unsupported)?;
        Ok(
            ProgramLocationInstance {
                ctx: self.ctx.clone(),
                location
            }
        )
    }

    pub async fn get_uniform_location(&self, name: String) -> Result<UniformLocationInstance, CallError> {
        let location = self.ctx.get_uniform_location(name).await
            .ok_or(CallError::Unsupported)?;
        Ok(
            UniformLocationInstance {
                ctx: self.ctx.clone(),
                location
            }
        )
    }

    pub async fn get_program_parameter(&self, pname: ShaderParameter) -> Result<ProgramParameterInstance, CallError> {
        let param = self.ctx.get_program_parameter(pname).await
            .ok_or(CallError::Unsupported)?;
        Ok(
            ProgramParameterInstance {
                ctx: self.ctx.clone(),
                param,
            }
        )
    }
}

impl Session
for ProgramInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::ProgramCreateShaderRequest>() => {
                let request: api::ProgramCreateShaderRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.create_shader(request.kind).await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(Box::new(session))))
            }
            topic if topic == type_name::<api::ProgramLinkProgramRequest>() => {
                let _request: api::ProgramLinkProgramRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.link_program().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ProgramUseProgramRequest>() => {
                let _request: api::ProgramUseProgramRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.use_program().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ProgramGetAttribLocationRequest>() => {
                let request: api::ProgramGetAttribLocationRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.get_attrib_location(request.name).await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(session)))
            }
            topic if topic == type_name::<api::ProgramGetUniformLocationRequest>() => {
                let request: api::ProgramGetUniformLocationRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.get_uniform_location(request-name).await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(session)))
            }
            topic if topic == type_name::<api::ProgramGetProgramParameterRequest>() => {
                let request: api::ProgramGetProgramParameterRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let session = self.get_program_parameter(request.pname).await?;
                Ok((ResultInvokable::new(SerializationFormat::Json, ()), Some(session)))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for ProgramInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let program = self.program.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_program(program).await;
        });
    }
}

pub struct ProgramParameterInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    param: ProgramParameterId,
}

impl ProgramParameterInstance {
}

impl Session
for ProgramParameterInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for ProgramParameterInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let param = self.param.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_program_parameter(param).await;
        });
    }
}

pub struct ProgramLocationInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    location: ProgramLocationId,
}

#[async_trait]
impl ProgramLocationInstance {
    pub async fn bind_program_location(&self) {
        self.ctx.bind_program_location(self.location).await;
    }

    pub async fn vertex_attrib_pointer(&self, size: AttributeSize, kind: DataType, normalized: bool, stride: u32, offset: u32) {
        self.ctx.vertex_attrib_pointer(self.location, size, kind, normalized, stride, offset).await;
    }

    pub async fn enable_vertex_attrib_array(&self) {
        self.ctx.enable_vertex_attrib_array(self.location);
    }
}

impl Session
for ProgramLocationInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::ProgramLocationBindProgramLocationRequest>() => {
                let _request: api::ProgramLocationBindProgramLocationRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.bind_program_location().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ProgramLocationVertexAttribPointerRequest>() => {
                let request: api::ProgramLocationVertexAttribPointerRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self. vertex_attrib_pointer(request.size, request.kind, request.normalized, request.stride, request.offset).await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ProgramLocationEnableVertexAttribArrayRequest>() => {
                let _request: api::ProgramLocationEnableVertexAttribArrayRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.enable_vertex_attrib_array().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for ProgramLocationInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let location = self.location.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_program_location(location).await;
        });
    }
}

pub struct VertexArrayInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    vertex_array: VertexArrayId
}

impl VertexArrayInstance {
    pub async fn bind_vertex_array(&self) {
        self.ctx.bind_vertex_array(self.vertex_array).await;
    }

    pub async fn unbind_vertex_array(&self) {
        self.ctx.unbind_vertex_array(self.vertex_array).await;
    }
}

impl Session
for VertexArrayInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::VertexArrayBindVertexArrayRequest>() => {
                let _request: api::VertexArrayBindVertexArrayRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.bind_vertex_array().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::VertexArrayUnbindVertexArrayRequest>() => {
                let _request: api::VertexArrayUnbindVertexArrayRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.unbind_vertex_array().await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for VertexArrayInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let vertex_array = self.vertex_array.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_vertex_array(vertex_array).await;
        });
    }
}

pub struct UniformLocationInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    location: UniformLocationId,
}

impl UniformLocationInstance {
    pub async fn uniform_matrix_4fv(&self, value: [[f32; 4]; 4]) {
        self.ctx.uniform_matrix_4fv(self.location, value).await;
    }

    pub async fn uniform_matrix_3fv(&self, value: [[f32; 3]; 3]) {
        self.ctx.uniform_matrix_3fv(self.location, value).await;
    }

    pub async fn uniform_matrix_2fv(&self, value: [[f32; 2]; 2]) {
        self.ctx.uniform_matrix_2fv(self.location, value).await;
    }

    pub async fn uniform_1i(&self, value: i32) {
        self.ctx.uniform_1i(self.location, value).await;
    }

    pub async fn uniform_1f(&self, value: f32) {
        self.ctx.uniform_1f(self.location, value).await;
    }

    pub async fn uniform_2f(&self, value: (f32, f32)) {
        self.ctx.uniform_2f(self.location, value).await;
    }

    pub async fn uniform_3f(&self, value: (f32, f32, f32)) {
        self.ctx.uniform_3f(self.location, value).await;
    }

    pub async fn uniform_4f(&self, value: (f32, f32, f32, f32)) {
        self.ctx.uniform_4f(self.location, value).await;
    }
}

impl Session
for UniformLocationInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::UniformLocationUniformMatrix4FvRequest>() => {
                let request: api::UniformLocationUniformMatrix4FvRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_matrix_4fv(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniformMatrix3FvRequest>() => {
                let request: api::UniformLocationUniformMatrix3FvRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_matrix_3fv(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniformMatrix2FvRequest>() => {
                let request: api::UniformLocationUniformMatrix2FvRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_matrix_2fv(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniform1IRequest>() => {
                let request: api::UniformLocationUniform1IRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_1i(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniform1FRequest>() => {
                let request: api::UniformLocationUniform1FRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_1f(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniform2FRequest>() => {
                let request: api::UniformLocationUniform2FRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_2f(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniform3FRequest>() => {
                let request: api::UniformLocationUniform3FRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_3f(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            topic if topic == type_name::<api::UniformLocationUniform4FRequest>() => {
                let request: api::UniformLocationUniform4FRequest = decode_request(SerializationFormat::Bincode, request.as_ref())?;
                let ret = self.uniform_4f(request.value).await;
                Ok((ResultInvokable::new(SerializationFormat::Bincode, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for UniformLocationInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let location = self.location.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_uniform_location(location).await;
        });
    }
}

pub struct ShaderInstance {
    ctx: Arc<Mutex<Box<dyn RenderingContextAbi>>>,
    shader: ShaderId,
}

impl ShaderInstance {
    pub async fn shader_source(&self, source: String) {
        self.ctx.shader_source(self.shader, source).await;
    }

    pub async fn shader_compile(&self) -> Result<(), String> {
        self.ctx.shader_compile(self.shader).await
    }

    pub async fn attach_shader(&self) -> Result<(), String> {
        self.ctx.attach_shader(self.shader).await
    }
}

impl Session
for ShaderInstance
{
    fn call(&mut self, topic: &str, request: Vec<u8>, _keepalive: bool) -> Result<(Box<dyn Invokable + 'static>, Option<Box<dyn Session + 'static>>), CallError> {
        match topic {
            topic if topic == type_name::<api::ShaderShaderSourceRequest>() => {
                let request: api::ShaderShaderSourceRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.shader_source(request.source).await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ShaderShaderCompileRequest>() => {
                let _request: api::ShaderShaderCompileRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.shader_compile().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            topic if topic == type_name::<api::ShaderAttachShaderRequest>() => {
                let _request: api::ShaderAttachShaderRequest = decode_request(SerializationFormat::Json, request.as_ref())?;
                let ret = self.attach_shader().await;
                Ok((ResultInvokable::new(SerializationFormat::Json, ret), None))
            }
            _ => Err(CallError::InvalidTopic)
        }
    }
}

impl Drop
for ShaderInstance
{
    fn drop(&mut self) {
        let ctx = self.ctx.clone();
        let shader = self.shader.clone();
        System::default().fork_shared(|| async move {
            ctx.delete_shader(shader).await;
        });
    }
}