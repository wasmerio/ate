use chrono::prelude::*;
use std::sync::Arc;
use term_lib::api::*;
use term_lib::common::MAX_MPSC;
use term_lib::console::Console;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use web_sys::KeyboardEvent;
use web_sys::WebGl2RenderingContext;
#[allow(unused_imports)]
use xterm_js_rs::addons::fit::FitAddon;
#[allow(unused_imports)]
use xterm_js_rs::addons::web_links::WebLinksAddon;
#[allow(unused_imports)]
use xterm_js_rs::addons::webgl::WebglAddon;
use xterm_js_rs::Theme;
use xterm_js_rs::{LogLevel, OnKeyEvent, Terminal, TerminalOptions};

use crate::system::TerminalCommand;
use crate::system::WebConsole;
use crate::system::WebSystem;

use super::common::*;
use super::pool::*;

#[macro_export]
#[doc(hidden)]
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

#[wasm_bindgen(start)]
pub fn main() {
    //let _ = console_log::init_with_level(log::Level::Debug);
    set_panic_hook();
}

#[derive(Debug)]
pub enum InputEvent {
    Key(KeyboardEvent),
    Command(String, Option<js_sys::Function>),
    Data(String),
}

#[wasm_bindgen]
pub struct ConsoleInput {
    tx: mpsc::Sender<InputEvent>,
    terminal: Terminal,
}

#[wasm_bindgen]
impl ConsoleInput {
    #[wasm_bindgen]
    pub fn send_command(&self, data: String, func: Option<js_sys::Function>) {
        self.tx
            .blocking_send(InputEvent::Command(data, func))
            .unwrap();
    }

    #[wasm_bindgen]
    pub fn send_data(&self, data: String) {
        self.tx.blocking_send(InputEvent::Data(data)).unwrap();
    }

    #[wasm_bindgen(method, getter)]
    pub fn terminal(&self) -> JsValue {
        self.terminal.clone()
    }
}

#[wasm_bindgen]
pub fn start(
    terminal_element: web_sys::Element,
    front_buffer: HtmlCanvasElement,
    init_command: Option<String>,
    on_ready: Option<js_sys::Function>,
) -> Result<ConsoleInput, JsValue> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = navigator, js_name = userAgent)]
        static USER_AGENT: String;
    }

    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_report_logs_in_timings(false)
            .set_max_level(tracing::Level::DEBUG)
            .build(),
    );

    info!("glue::start");

    let terminal = Terminal::new(
        TerminalOptions::new()
            .with_log_level(LogLevel::Info)
            .with_rows(50)
            .with_cursor_blink(true)
            .with_cursor_width(10)
            .with_font_size(16u32)
            .with_draw_bold_text_in_bright_colors(true)
            .with_right_click_selects_word(true)
            .with_transparency(true)
            .with_font_size(17)
            .with_font_family("Zeitung Mono Pro")
            .with_theme(
                &Theme::new()
                    .with_background("#23104400")
                    .with_foreground("#ffffff")
                    .with_black("#fdf6e3")
                    .with_green("#02C39A")
                    .with_cyan("#4AB3FF"),
            ),
    );

    let window = web_sys::window().unwrap();
    let location = window.location().href().unwrap();

    let user_agent = USER_AGENT.clone();
    let is_mobile = term_lib::common::is_mobile(&user_agent);
    debug!("user_agent: {}", user_agent);

    let elem = window
        .document()
        .unwrap()
        .get_element_by_id("terminal")
        .unwrap();

    terminal.open(elem.clone().dyn_into()?);

    let (term_tx, mut term_rx) = mpsc::channel(MAX_MPSC);
    {
        let terminal: Terminal = terminal.clone().dyn_into().unwrap();
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(cmd) = term_rx.recv().await {
                match cmd {
                    TerminalCommand::Print(text) => {
                        terminal.write(text.as_str());
                    }
                    TerminalCommand::ConsoleRect(tx) => {
                        let _ = tx
                            .send(ConsoleRect {
                                cols: terminal.get_cols(),
                                rows: terminal.get_rows(),
                            })
                            .await;
                    }
                    TerminalCommand::Cls => {
                        terminal.clear();
                    }
                }
            }
        });
    }

    let front_buffer = window
        .document()
        .unwrap()
        .get_element_by_id("frontBuffer")
        .unwrap();
    let front_buffer: HtmlCanvasElement = front_buffer
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let webgl2 = front_buffer
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let pool = WebThreadPool::new_with_max_threads().unwrap();
    let web_system = WebSystem::new(pool.clone(), webgl2);
    let web_console = WebConsole::new(term_tx);
    term_lib::api::set_system_abi(web_system);
    let system = System::default();

    let fs = term_lib::fs::create_root_fs(None);
    let mut console = Console::new(
        location,
        user_agent,
        term_lib::eval::Compiler::Default,
        Arc::new(web_console),
        None,
        fs,
    );
    let tty = console.tty().clone();

    let (tx, mut rx) = mpsc::channel(MAX_MPSC);

    let tx_key = tx.clone();
    let callback = {
        Closure::wrap(Box::new(move |e: OnKeyEvent| {
            let event = e.dom_event();
            tx_key.blocking_send(InputEvent::Key(event)).unwrap();
        }) as Box<dyn FnMut(_)>)
    };
    terminal.on_key(callback.as_ref().unchecked_ref());
    callback.forget();

    let tx_data = tx.clone();
    let callback = {
        Closure::wrap(Box::new(move |data: String| {
            tx_data.blocking_send(InputEvent::Data(data)).unwrap();
        }) as Box<dyn FnMut(_)>)
    };
    terminal.on_data(callback.as_ref().unchecked_ref());
    callback.forget();

    /*
    {
        let addon = FitAddon::new();
        terminal.load_addon(addon.clone().dyn_into::<FitAddon>()?.into());
        addon.fit();
    }
    */

    /*
    {
        let addon = WebLinksAddon::new();
        terminal.load_addon(addon.clone().dyn_into::<WebLinksAddon>()?.into());
        addon.fit();
    }
    */

    /*
    {
        let addon = WebglAddon::new(None);
        terminal.load_addon(addon.clone().dyn_into::<WebglAddon>()?.into());
    }
    */

    {
        let front_buffer: HtmlCanvasElement = front_buffer.clone().dyn_into().unwrap();
        let terminal: Terminal = terminal.clone().dyn_into().unwrap();
        term_fit(terminal, front_buffer);
    }

    {
        let front_buffer: HtmlCanvasElement = front_buffer.clone().dyn_into().unwrap();
        let terminal: Terminal = terminal.clone().dyn_into().unwrap();
        let closure = {
            Closure::wrap(Box::new(move || {
                let front_buffer: HtmlCanvasElement = front_buffer.clone().dyn_into().unwrap();
                let terminal: Terminal = terminal.clone().dyn_into().unwrap();
                term_fit(
                    terminal.clone().dyn_into().unwrap(),
                    front_buffer.clone().dyn_into().unwrap(),
                );

                let tty = tty.clone();
                system.fork_local(async move {
                    let cols = terminal.get_cols();
                    let rows = terminal.get_rows();
                    tty.set_bounds(cols, rows).await;
                });
            }) as Box<dyn FnMut()>)
        };
        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        window.add_event_listener_with_callback(
            "orientationchange",
            closure.as_ref().unchecked_ref(),
        )?;
        closure.forget();
    }

    terminal.focus();

    system.fork_local(async move {
        console.init(init_command, on_ready).await;

        crate::glue::show_terminal();

        let mut last = None;
        while let Some(event) = rx.recv().await {
            match event {
                InputEvent::Key(event) => {
                    console
                        .on_key(
                            event.key_code(),
                            event.key(),
                            event.alt_key(),
                            event.ctrl_key(),
                            event.meta_key(),
                        )
                        .await;
                }
                InputEvent::Command(data, func) => {
                    console.on_data(data).await;
                    console.on_enter_with_callback(func).await
                }
                InputEvent::Data(data) => {
                    // Due to a nasty bug in xterm.js on Android mobile it sends the keys you press
                    // twice in a row with a short interval between - this hack will avoid that bug
                    if is_mobile {
                        let now: DateTime<Local> = Local::now();
                        let now = now.timestamp_millis();
                        if let Some((what, when)) = last {
                            if what == data && now - when < 200 {
                                last = None;
                                continue;
                            }
                        }
                        last = Some((data.clone(), now))
                    }

                    console.on_data(data).await;
                }
            }
        }
    });

    Ok(ConsoleInput { tx: tx, terminal })
}

#[wasm_bindgen(module = "/js/fit.ts")]
extern "C" {
    #[wasm_bindgen(js_name = "termFit")]
    fn term_fit(terminal: Terminal, front: HtmlCanvasElement);
}

#[wasm_bindgen(module = "/js/gl.js")]
extern "C" {
    #[wasm_bindgen(js_name = "showTerminal")]
    pub fn show_terminal();
    #[wasm_bindgen(js_name = "showCanvas")]
    pub fn show_canvas();
}
