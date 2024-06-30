use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
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

#[derive(Debug)]
pub struct Tail {
    pub cmd_tx: Sender<TailMessage>,
    pub cmd_rx: Arc<Receiver<TailMessage>>,
    pub tx: Sender<String>,
}

impl Tail {
    pub fn new(outer_tx: Sender<String>) -> Self {
        let (tx, rx) = channel::<TailMessage>();

        Self {
            cmd_tx: tx,
            cmd_rx: Arc::new(rx),
            tx: outer_tx,
        }
    }

    pub fn start_listen(self) {
        let mut cmd_rx = self.cmd_rx.clone();

        thread::Builder::new()
            .name("Tail file reader".into())
            .spawn(move || {
                let mut limiter = CpuLimiter::new(Duration::from_millis(150));

                let mut logfile: Option<File> = None;
                loop {
                    match cmd_rx.try_recv() {
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
                        let mut buf: &mut String = &mut Default::default();

                        file.read_to_string(buf);

                        self.tx.send(buf.to_string());

                        file.seek(SeekFrom::Current(0)).unwrap();
                    }

                    limiter.might_sleep();
                }
            })
            .unwrap();
    }

    pub fn stop(self) {
        self.cmd_tx.send(TailMessage::Stop).unwrap();
    }

    pub fn open_file(self, path: PathBuf) {
        self.cmd_tx.send(TailMessage::OpenNewFile(path)).unwrap();
    }
}
