use crate::clocks::WasiClocks;
use crate::dir::{DirEntry, WasiDir};
use crate::file::WasiFile;
use crate::sched::WasiSched;
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use crate::Error;
use cap_rand::RngCore;
use std::path::{Path, PathBuf};

pub struct WasiCtx {
    pub args: StringArray,
    pub env: StringArray,
    pub random: Box<dyn RngCore + Send + Sync>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
    pub table: Table,
}

impl WasiCtx {
    pub fn new(
        random: Box<dyn RngCore + Send + Sync>,
        clocks: WasiClocks,
        sched: Box<dyn WasiSched>,
        table: Table,
    ) -> Self {
        let mut s = WasiCtx {
            args: StringArray::new(),
            env: StringArray::new(),
            random,
            clocks,
            sched,
            table,
        };
        s.set_stdin(Box::new(crate::pipe::ReadPipe::new(std::io::empty())));
        s.set_stdout(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s.set_stderr(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s
    }

    pub fn insert_file(&mut self, fd: u32, file: Box<dyn WasiFile>) {
        self.table().insert_at(fd, Box::new(file));
    }

    pub fn push_file(&mut self, file: Box<dyn WasiFile>) -> Result<u32, Error> {
        self.table().push(Box::new(file))
    }

    pub fn insert_dir(&mut self, fd: u32, dir: Box<dyn WasiDir>, path: PathBuf) {
        self.table()
            .insert_at(fd, Box::new(DirEntry::new(Some(path), dir)));
    }

    pub fn push_dir(&mut self, dir: Box<dyn WasiDir>, path: PathBuf) -> Result<u32, Error> {
        self.table().push(Box::new(DirEntry::new(Some(path), dir)))
    }

    pub fn table(&mut self) -> &mut Table {
        &mut self.table
    }

    pub fn push_arg(&mut self, arg: &str) -> Result<(), StringArrayError> {
        self.args.push(arg.to_owned())
    }

    pub fn push_env(&mut self, var: &str, value: &str) -> Result<(), StringArrayError> {
        self.env.push(format!("{}={}", var, value))?;
        Ok(())
    }

    pub fn set_stdin(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(0, f);
    }

    pub fn set_stdout(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(1, f);
    }

    pub fn set_stderr(&mut self, f: Box<dyn WasiFile>) {
        self.insert_file(2, f);
    }

    pub fn push_preopened_dir(
        &mut self,
        dir: Box<dyn WasiDir>,
        path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        self.table()
            .push(Box::new(DirEntry::new(Some(path.as_ref().to_owned()), dir)))?;
        Ok(())
    }
}
