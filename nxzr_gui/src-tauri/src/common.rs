use std::io;
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone)]
pub struct TracingWriterChannel {
    writer_tx: mpsc::UnboundedSender<String>,
}

impl TracingWriterChannel {
    pub fn new(writer_tx: mpsc::UnboundedSender<String>) -> Self {
        Self { writer_tx }
    }
}

impl io::Write for TracingWriterChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let json = String::from_utf8_lossy(buf).into_owned();
        let _ = self.writer_tx.send(json);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TracingWriterChannel {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
