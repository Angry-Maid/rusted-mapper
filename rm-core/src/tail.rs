use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use log::{debug, info};
use might_sleep::cpu_limiter::CpuLimiter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum TailCmd {
    Open(PathBuf),
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TailMsg {
    Content(String),
    NewFile,
    Stop,
}

#[derive(Debug, Clone, Copy)]
pub struct Tail {}

impl Tail {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start_listen(&mut self) -> anyhow::Result<(Sender<TailCmd>, Receiver<TailMsg>)> {
        let (command_tx, command_rx) = channel::<TailCmd>();
        let (data_tx, data_rx) = channel::<TailMsg>();

        thread::Builder::new()
            .name("tail file reader".into())
            .spawn(|| Tail::tail_file(command_rx, data_tx))?;

        Ok((command_tx, data_rx))
    }

    pub fn tail_file(
        command_rx: Receiver<TailCmd>,
        data_tx: Sender<TailMsg>,
    ) -> anyhow::Result<()> {
        let mut limiter = CpuLimiter::new(Duration::from_millis(250));

        let mut logfile: Option<File> = None;
        loop {
            match command_rx.try_recv() {
                Ok(val) => match val {
                    TailCmd::Open(filepath) => {
                        logfile.replace(File::open(filepath)?);
                        data_tx.send(TailMsg::NewFile)?;
                    }
                    TailCmd::Stop => {
                        data_tx.send(TailMsg::Stop);
                        info!("Tail channel got command stop, stopping thread.");
                        break;
                    }
                },
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    debug!("Tail channel was disconnected");
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
