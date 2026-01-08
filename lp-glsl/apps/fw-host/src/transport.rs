//! Host transport implementation using stdio

use lp_core::error::Error;
use lp_core::traits::Transport;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc;
use std::thread;

/// Host transport implementation using stdin/stdout
pub struct HostTransport {
    stdout: io::Stdout,
    receiver: Option<mpsc::Receiver<String>>,
}

impl HostTransport {
    /// Create a new host transport
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        // Spawn a background thread to read from stdin
        thread::spawn(move || {
            let stdin = BufReader::new(io::stdin());
            for line in stdin.lines() {
                match line {
                    Ok(msg) => {
                        if tx.send(msg).is_err() {
                            break; // Receiver dropped, exit thread
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            stdout: io::stdout(),
            receiver: Some(rx),
        }
    }
}

impl Default for HostTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for HostTransport {
    fn send_message(&mut self, message: &str) -> Result<(), Error> {
        let stdout = &mut self.stdout;
        stdout
            .write_all(message.as_bytes())
            .map_err(|e| Error::Protocol(format!("Failed to write to stdout: {}", e)))?;
        stdout
            .write_all(b"\n")
            .map_err(|e| Error::Protocol(format!("Failed to write newline: {}", e)))?;
        stdout
            .flush()
            .map_err(|e| Error::Protocol(format!("Failed to flush stdout: {}", e)))?;
        Ok(())
    }

    fn receive_message(&mut self) -> Result<String, Error> {
        // Try to receive from the background thread (non-blocking)
        match self.receiver.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(msg) => Ok(msg),
                Err(mpsc::TryRecvError::Empty) => {
                    // No data available, return an error that indicates this
                    Err(Error::Protocol("No message available".to_string()))
                }
                Err(mpsc::TryRecvError::Disconnected) => Err(Error::Protocol(
                    "Stdin reader thread disconnected".to_string(),
                )),
            },
            None => Err(Error::Protocol("No receiver available".to_string())),
        }
    }
}
