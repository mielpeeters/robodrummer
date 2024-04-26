use std::{sync::mpsc::Receiver, time::Duration};

/// Non-blocking receiving calls to the receiver until the last value has been retreived
#[allow(dead_code)]
pub fn get_last_sent<T>(rx: &Receiver<T>) -> Option<T> {
    let mut last = None;
    while let Ok(val) = rx.try_recv() {
        last = Some(val);
    }
    last
}

/// Non-blocking receiving calls to the receiver until the last value has been retreived
pub fn get_last_sent_timeout<T>(rx: &Receiver<T>, timeout: Duration) -> Option<T>
where
    T: std::fmt::Debug,
{
    let mut last = None;
    if let Ok(m) = rx.recv_timeout(timeout) {
        last = Some(m);
        while let Ok(val) = rx.try_recv() {
            last = Some(val);
        }
    }
    last
}
