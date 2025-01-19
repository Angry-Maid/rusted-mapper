use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use log::{debug, info};
use might_sleep::prelude::CpuLimiter;

#[derive(Debug)]
pub enum TailCmd {
    Open(PathBuf),
    ForceUpdate,
    Stop,
}

#[derive(Debug, Clone)]
pub enum TailMsg {
    Content(String),
    NewFile,
    Stop,
}

#[derive(Clone, Copy)]
pub struct Tail;

impl Tail {
    pub fn start_listen(
        command_rx: Receiver<TailCmd>,
        data_tx: Sender<TailMsg>,
    ) -> anyhow::Result<()> {
        thread::Builder::new()
            .name("tail file reader".into())
            .spawn(|| Tail::tail(command_rx, data_tx))?;

        Ok(())
    }

    pub fn tail(command_rx: Receiver<TailCmd>, data_tx: Sender<TailMsg>) -> anyhow::Result<()> {
        let mut limiter = CpuLimiter::new(Duration::from_millis(250));

        let mut logfile: Option<File> = None;

        loop {
            match command_rx.try_recv() {
                Ok(val) => match val {
                    TailCmd::Open(filepath) => {
                        logfile.replace(File::open(filepath)?);
                        data_tx.send(TailMsg::NewFile)?;
                    }
                    TailCmd::ForceUpdate => data_tx.send(TailMsg::Content("".into()))?,
                    TailCmd::Stop => {
                        data_tx.send(TailMsg::Stop)?;
                        info!("Tail got: {:?}", TailCmd::Stop);
                        break;
                    }
                },
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    debug!("Tail command channel was disconnected");
                    break;
                }
            }

            if let Some(ref mut file) = logfile {
                let buf: &mut String = &mut Default::default();

                file.read_to_string(buf)?;

                if !buf.is_empty() {
                    data_tx.send(TailMsg::Content(buf.to_string()))?;
                }

                file.seek(SeekFrom::Current(0))?;
            }

            limiter.might_sleep();
        }

        Ok(())
    }
}
