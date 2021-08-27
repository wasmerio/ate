#![allow(unused_imports, dead_code)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use async_trait::async_trait;
use pbr::ProgressBar;
use pbr::Units;
use std::io::Stderr;

use crate::mesh::Loader;
use crate::event::EventData;
use crate::error::ChainCreationError;
use crate::loader::LoadData;

pub struct LoadProgress
{
    pub msg_done: String,
    pub units: pbr::Units,
    pub bar: Option<ProgressBar<Stderr>>,
}

impl Default
for LoadProgress
{
    fn default() -> LoadProgress {
        LoadProgress {
            msg_done: "Done".to_string(),
            units: pbr::Units::Default,
            bar: None,
        }
    }
}

#[async_trait]
impl Loader
for LoadProgress
{
    fn human_message(&mut self, message: String) {
        if self.bar.is_some() {
            self.msg_done.push_str("\n");
            self.msg_done.push_str(message.as_str());
        } else {
            eprintln!("{}", message);
        }
    }

    async fn start_of_history(&mut self, size: usize)
    {
        let handle = ::std::io::stderr();
        let mut pb = ProgressBar::on(handle, size as u64);
        match &self.units {
            Units::Default => pb.set_units(Units::Default),
            Units::Bytes => pb.set_units(Units::Bytes),
        }
        pb.format("╢█▌░╟");
        self.bar.replace(pb);
    }

    fn feed_events(&mut self, evts: &Vec<EventData>)
    {
        if let Some(pb) = &mut self.bar {
            pb.add(evts.len() as u64);
        }
    }

    async fn feed_load_data(&mut self, data: LoadData)
    {
        if let Some(pb) = &mut self.bar {
            let total = 2 + data.header.meta_bytes.len() + match data.data.data_bytes {
                Some(a) => a.len(),
                None => 0
            };
            pb.add(total as u64);
        }
    }

    async fn end_of_history(&mut self)
    {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print(self.msg_done.as_str());
        }
    }

    async fn failed(&mut self, err: ChainCreationError) -> Option<ChainCreationError>
    {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print(err.to_string().as_str());
        } else {
            error!("{}", err.to_string());
        }
        Some(err)
    }
}