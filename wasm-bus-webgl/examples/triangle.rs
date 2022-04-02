use wasm_bus_webgl::prelude::*;

fn main() -> Result<(), WebGlError> {
    let context = WebGl2::new()?;

    let program = context
        .create_program();

    let vert_shader = compile_shader(
        &program,
        ShaderKind::Vertex,
        r##"#version 300 es
 
        in vec4 position;

        void main() {
        
            gl_Position = position;
        }
        "##,
    )?;

    let frag_shader = compile_shader(
        &program,
        ShaderKind::Fragment,
        r##"#version 300 es
    
        precision highp float;
        out vec4 outColor;
        
        void main() {
            outColor = vec4(1, 1, 1, 1);
        }
        "##,
    )?;
    
    link_program(&program, &vert_shader, &frag_shader)?;
    
    program.use_program();

    let vertices: [f32; 9] = [-0.7, -0.7, 0.0, 0.7, -0.7, 0.0, 0.0, 0.7, 0.0];

    let position_attribute_location = program
        .get_attrib_location("position");

    let buffer = context.create_buffer();    
    buffer.bind(BufferKind::Array);

    context.buffer_data_f32(BufferKind::Array, &vertices[..], DrawMode::Static);

    let vao = context
        .create_vertex_array();
    vao.bind();

    position_attribute_location.vertex_attrib_pointer(AttributeSize::Three, DataType::Float, false, 0, 0);
    position_attribute_location.enable();

    vao.bind();

    let vert_count = (vertices.len() / 3) as i32;
    draw(&context, vert_count);

    std::thread::sleep(std::time::Duration::from_secs(4));

    Ok(())
}

#[allow(dead_code)]
fn draw(context: &WebGl2, vert_count: i32) {
    context.clear_color(0.0, 0.0, 0.4, 1.0);
    context.clear(BufferBit::Color);
    context.draw_arrays(Primitives::Triangles, 0, vert_count);
}

#[allow(dead_code)]
fn compile_shader(
    program: &Program,
    shader_type: ShaderKind,
    source: &str,
) -> Result<Shader, WebGlError>
{
    let shader = program.create_shader(shader_type);    
    shader.set_source(source);    
    shader.compile()?;
    Ok(shader)
}

#[allow(dead_code)]
pub fn link_program(
    program: &Program,
    vert_shader: &Shader,
    frag_shader: &Shader,
) -> Result<(), WebGlError>
{    
    vert_shader.attach()?;
    frag_shader.attach()?;
    program.link()?;
    Ok(())
}