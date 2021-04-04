#![allow(unused_imports, dead_code)]
use log::{info, error, debug};
use async_trait::async_trait;
use pbr::ProgressBar;
use ate::mesh::Loader;
use std::io::Stderr;
use ate::event::EventData;
use ate::error::ChainCreationError;

#[derive(Default)]
pub struct LoadProgress
{
    bar: Option<ProgressBar<Stderr>>,
}

#[async_trait]
impl Loader
for LoadProgress
{
    async fn start_of_history(&mut self, size: usize)
    {
        let handle = ::std::io::stderr();
        let mut pb = ProgressBar::on(handle, size as u64);
        pb.format("╢▌▌░╟");
        self.bar.replace(pb);
    }

    async fn feed_events(&mut self, evts: &Vec<EventData>)
    {
        if let Some(pb) = &mut self.bar {
            pb.add(evts.len() as u64);
        }
    }

    async fn end_of_history(&mut self)
    {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print("done");
        }
    }

    async fn failed(&mut self, err: ChainCreationError) -> Option<ChainCreationError>
    {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print("failed");
        }
        error!("{}", err.to_string());
        Some(err)
    }
}