use std::io;

use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone)]
pub struct WriterChannel {
    writer_tx: mpsc::UnboundedSender<String>,
}

impl WriterChannel {
    pub fn new(writer_tx: mpsc::UnboundedSender<String>) -> Self {
        Self { writer_tx }
    }
}

impl io::Write for WriterChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let json = String::from_utf8_lossy(buf).into_owned();
        let _ = self.writer_tx.send(json);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for WriterChannel {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
