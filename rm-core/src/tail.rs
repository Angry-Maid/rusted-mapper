use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread,
    time::Duration,
};

use log::{debug, info};
use might_sleep::cpu_limiter::CpuLimiter;

#[derive(Debug, Clone)]
pub enum TailMessage {
    OpenNewFile(PathBuf),
    Stop,
}

#[derive(Debug, Clone, Copy)]
pub struct Tail;

impl Tail {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start_listen(&mut self) -> Result<(Sender<TailMessage>, Receiver<String>), io::Error> {
        let (command_tx, command_rx) = channel::<TailMessage>();
        let (data_tx, data_rx) = channel::<String>();

        thread::Builder::new()
            .name("Tail file reader".into())
            .spawn(|| Tail::tail_file(command_rx, data_tx))?;

        Ok((command_tx, data_rx))
    }

    pub fn tail_file(command_receiver: Receiver<TailMessage>, data_tranceiver: Sender<String>) {
        let mut limiter = CpuLimiter::new(Duration::from_millis(150));

        let mut logfile: Option<File> = None;
        loop {
            match command_receiver.try_recv() {
                Ok(val) => match val {
                    TailMessage::OpenNewFile(filepath) => {
                        logfile.replace(File::open(filepath).unwrap());
                    }
                    TailMessage::Stop => {
                        info!("Tail channel got command stop, stopping thread.");
                        break;
                    }
                },
                Err(TryRecvError::Empty) => {}
                Err(e) => {
                    debug!("Tail channel was disconnected - {e:?}");
                    break;
                }
            }

            if let Some(ref mut file) = logfile {
                let buf: &mut String = &mut Default::default();

                file.read_to_string(buf);

                data_tranceiver.send(buf.to_string());

                file.seek(SeekFrom::Current(0)).unwrap();
            }

            limiter.might_sleep();
        }
    }
}
