#![allow(unused_imports, dead_code)]
use async_trait::async_trait;
use pbr::ProgressBar;
use pbr::Units;
use std::io::Write;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::error::ChainCreationError;
use crate::event::EventData;
use crate::loader::LoadData;
use crate::mesh::Loader;

pub struct LoadProgress<T>
where T: Write + Send + Sync {
    pub msg_done: String,
    pub units: pbr::Units,
    pub bar: Option<ProgressBar<T>>,
    pub writer: Option<T>,
}

impl<T> LoadProgress<T>
where T: Write + Send + Sync {
    pub fn new(writer: T) -> LoadProgress<T> {
        LoadProgress {
            msg_done: "Done".to_string(),
            units: pbr::Units::Default,
            bar: None,
            writer: Some(writer)
        }
    }
}

#[async_trait]
impl<T> Loader
for LoadProgress<T>
where T: Write + Send + Sync
{
    fn human_message(&mut self, message: String) {
        if self.bar.is_some() {
            self.msg_done.push_str("\n");
            self.msg_done.push_str(message.as_str());
        } else if let Some(writer) = self.writer.as_mut() {
            let message = message.into_bytes();
            let _ = writer.write_all(&message[..]);
        }
    }

    async fn start_of_history(&mut self, size: usize) {
        if let Some(writer) = self.writer.take() {
            let mut pb = ProgressBar::on(writer, size as u64);
            match &self.units {
                Units::Default => pb.set_units(Units::Default),
                Units::Bytes => pb.set_units(Units::Bytes),
            }
            pb.format("╢█▌░╟");
            self.bar.replace(pb);
        }
    }

    fn feed_events(&mut self, evts: &Vec<EventData>) {
        if let Some(pb) = &mut self.bar {
            pb.add(evts.len() as u64);
        }
    }

    async fn feed_load_data(&mut self, data: LoadData) {
        if let Some(pb) = &mut self.bar {
            let total = 2
                + data.header.meta_bytes.len()
                + match data.data.data_bytes {
                    Some(a) => a.len(),
                    None => 0,
                };
            pb.add(total as u64);
        }
    }

    async fn end_of_history(&mut self) {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print(self.msg_done.as_str());
        }
    }

    async fn failed(&mut self, err: ChainCreationError) -> Option<ChainCreationError> {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print(err.to_string().as_str());
        } else if let Some(writer) = self.writer.as_mut() {
            let message = err.to_string().into_bytes();
            let _ = writer.write_all(&message[..]);
        }
        Some(err)
    }
}
