use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;
use std::collections::HashMap;

use actix_web::web::{Bytes, Data};
use actix_web::{Error};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{interval_at, Instant};

pub fn broadcast(
    event: String,
    msg: String,
    broadcaster: Data<Mutex<Broadcaster>>,
) -> () {
    broadcaster.lock().unwrap().send(&event, &msg);
}


pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    pub fn create() -> Data<Mutex<Self>> {
        // Data ≃ Arc
        let me = Data::new(Mutex::new(Broadcaster::new()));

        // ping clients every 10 seconds to see if they are alive
        Broadcaster::spawn_ping(me.clone());

        me
    }

    pub fn new() -> Self {
        Broadcaster {
            clients: Vec::new()
        }
    }

    pub fn spawn_ping(me: Data<Mutex<Self>>) {
        actix_rt::spawn(async move {
            let mut task = interval_at(Instant::now(), Duration::from_secs(10));
            while let Some(_) = task.next().await {
                me.lock().unwrap().remove_stale_clients();
            }
        })
    }

    pub fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Bytes::from("event: internal_status\ndata: ping\n\n"));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    pub fn new_client(&mut self, collection: HashMap<String, String>) -> Client {
        let (tx, rx) = channel(100);
        let tx_clone = tx.clone();

        for (evt, msg) in collection {
            let msg = Bytes::from(["event: ", &evt, "\ndata: ", &msg, "\n\n"].concat());

            tx_clone.clone()
                .try_send(msg)
                .unwrap();

            self.clients.push(tx_clone.clone());
        }
        Client(rx)
    }

    pub fn send(&self, evt: &str, msg: &str) {
        let msg = Bytes::from(["event: ", evt, "\n", "data: ", msg, "\n\n"].concat());

        for client in self.clients.iter() {
            client.clone().try_send(msg.clone()).unwrap_or(());
        }
    }
}

// wrap Receiver in own type, with correct error type
pub struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
