use std::{
    fs::OpenOptions,
    io::{self, ErrorKind},
    path::Path,
};

use super::{buffer::Buffer, persistence::write_document, Editor};

impl Editor {
    /// open a file or start blank if missing, preserving the document on errors
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let buffer = OpenOptions::new()
            .read(true)
            .open(path.as_ref())
            .and_then(Buffer::read_from);

        let buffer = match buffer {
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            r => Some(r?),
        };

        self.clear();
        self.opts.file = Some(path.as_ref().into());
        if let Some(buf) = buffer {
            self.lsave = buf.clone();
            self.buffer = buf;
        }

        Ok(())
    }

    /// save cleaned content, applying cleanup to the document only on success
    /// return false when no save path is set
    pub fn save(&mut self) -> io::Result<bool> {
        let Some(path) = self.path().cloned() else {
            return Ok(false);
        };
        let mut staged = self.buffer.clone();
        staged.discard_edits();
        if self.opts.strip_margin_ws {
            staged.strip_margin_whitespace();
        } else if !self.opts.keep_trailing_ws {
            staged.strip_trailing_whitespace();
        }
        let rendered: String = staged.iter("").collect();
        write_document(&path, rendered.as_bytes())?;
        self.with_snapshot(|editor| editor.buffer = staged);
        self.lsave = self.buffer.clone();
        self.dirty = false;
        Ok(true)
    }

    /// save at a new path, keeping the old path on failure
    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let old_path = self.opts.file.replace(path.as_ref().into());
        match self.save() {
            Ok(true) => Ok(()),
            Ok(false) => unreachable!("save_as always configures a path"),
            Err(error) => {
                self.opts.file = old_path;
                Err(error)
            }
        }
    }

    /// export committed content with trimmed margins, leaving the document unchanged
    pub fn export_text(&self, prefix: &str) -> String {
        let mut buf = self.buffer.clone();
        buf.strip_margin_whitespace();

        let mut rendered: String = buf.iter(prefix).collect();
        rendered.pop();
        rendered
    }
}
