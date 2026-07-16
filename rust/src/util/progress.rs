use std::io::{self, Write};

pub struct ProgressWriter<W: Write> {
    inner: W,
    total: u64,
    done: u64,
    last_pct: i32,
}

impl<W: Write> ProgressWriter<W> {
    pub fn new(inner: W, total: u64) -> Self {
        Self {
            inner,
            total,
            done: 0,
            last_pct: -1,
        }
    }

    fn tick(&mut self) {
        if self.total == 0 {
            return;
        }
        let pct = ((self.done * 100) / self.total) as i32;
        if pct != self.last_pct {
            self.last_pct = pct;
            let _ = write!(
                io::stderr(),
                "\r{}% ({}/{})",
                pct,
                crate::util::size::format_byte_size(self.done),
                crate::util::size::format_byte_size(self.total)
            );
            let _ = io::stderr().flush();
        }
    }
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.done += n as u64;
        self.tick();
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub fn finish_progress() {
    let _ = writeln!(io::stderr());
}
