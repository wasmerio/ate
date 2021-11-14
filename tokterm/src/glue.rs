#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;
use wasm_bindgen_futures::spawn_local;
use tokio::sync::mpsc;

#[allow(unused_imports)]
use xterm_js_rs::addons::fit::FitAddon;
#[allow(unused_imports)]
use xterm_js_rs::addons::web_links::WebLinksAddon;
#[allow(unused_imports)]
use xterm_js_rs::addons::webgl::WebglAddon;

use xterm_js_rs::{OnKeyEvent, LogLevel, Terminal, TerminalOptions};

use super::common::*;
use xterm_js_rs::{Theme};
use super::console::Console;
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
    Data(String),
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    //ate::log_init(0i32, false);
    tracing_wasm::set_as_global_default_with_config(tracing_wasm::WASMLayerConfigBuilder::new()
        .set_report_logs_in_timings(false)
        .set_max_level(tracing::Level::DEBUG)
        .build());

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
            .with_theme(
                &Theme::new(),
            ),
    );

    let window = web_sys::window()
        .unwrap();
    let elem = window
        .document()
        .unwrap()
        .get_element_by_id("terminal")
        .unwrap();

    terminal.open(elem.clone().dyn_into()?);

    let pool = ThreadPool::new_with_max_threads().unwrap();

    let mut console = Console::new(
        terminal.clone().dyn_into().unwrap(),
        pool
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

    spawn_local(async move {
        console.init().await;
        while let Some(event) = rx.recv().await {
            match event {
                InputEvent::Key(event) => {
                    console.on_key(event.key_code(), event.key(), event.alt_key(), event.ctrl_key(), event.meta_key()).await;
                },
                InputEvent::Data(data) => {
                    console.on_data(data).await;
                },
            }
        }
    });

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
        let terminal: Terminal = terminal.clone().dyn_into().unwrap();
        term_fit(terminal.clone().dyn_into().unwrap());
    }

    {
        let terminal: Terminal = terminal.clone().dyn_into().unwrap();
        let closure: Closure<dyn FnMut()> = Closure::new(
            move || {
                let terminal: Terminal = terminal.clone().dyn_into().unwrap();
                term_fit(terminal.clone().dyn_into().unwrap());
                
                let tty = tty.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let cols = terminal.get_cols();
                    let rows = terminal.get_rows();
                    tty.set_bounds(cols, rows).await;  
                });
            });
        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    
    terminal.focus();

    Ok(())
}

#[wasm_bindgen(module = "/src/js/fit.ts")]
extern "C" {
    #[wasm_bindgen(js_name = "termFit")]
    fn term_fit(
        terminal: Terminal,
    );
}