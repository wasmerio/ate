use pbr::ProgressBar;
use ate::mesh::Loader;
use std::io::Stderr;
use ate::event::EventData;

#[derive(Default)]
pub struct LoadProgress
{
    bar: Option<ProgressBar<Stderr>>,
}

impl Loader
for LoadProgress
{
    fn start_of_history(&mut self, size: usize)
    {
        let handle = ::std::io::stderr();
        let mut pb = ProgressBar::on(handle, size as u64);
        pb.format("╢▌▌░╟");
        self.bar.replace(pb);
    }

    fn feed_events(&mut self, evts: &Vec<EventData>)
    {
        if let Some(pb) = &mut self.bar {
            pb.add(evts.len() as u64);
        }
    }

    fn end_of_history(&mut self)
    {
        if let Some(mut pb) = self.bar.take() {
            pb.finish_print("done");
        }
    }
}