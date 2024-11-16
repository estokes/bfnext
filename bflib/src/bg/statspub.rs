use netidx_archive::{logfile::ArchiveWriter, recorder::Recorder};

pub struct Statspub {
    log: ArchiveWriter,
    publisher: Option<Recorder>
}

