extern crate alloc;
use crate::*;

use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, Duration};

use tracing::*;

use std::error::Error;
use std::result::Result;
use std::sync::Arc;

use alloc::vec::Vec;
use postcard::*;
use std::marker::PhantomData;

use crate::msg::*;
use chrono::{DateTime, Utc};

impl<T: Message> From<Node<Idle, T>> for Node<Active, T> {
    fn from(node: Node<Idle, T>) -> Self {
        Self {
            __state: PhantomData,
            phantom: PhantomData,
            runtime: node.runtime,
            stream: node.stream,
            name: node.name,
            topic: node.topic,
            host_addr_tcp: node.host_addr_tcp,
            host_addr_udp: node.host_addr_udp,
            socket: node.socket,
            subscription_data: node.subscription_data,
            task_subscribe: None,
        }
    }
}

impl<T: Message> From<Node<Idle, T>> for Node<Subscription, T> {
    fn from(node: Node<Idle, T>) -> Self {
        Self {
            __state: PhantomData,
            phantom: PhantomData,
            runtime: node.runtime,
            stream: node.stream,
            name: node.name,
            topic: node.topic,
            host_addr_tcp: node.host_addr_tcp,
            host_addr_udp: node.host_addr_udp,
            socket: node.socket,
            subscription_data: node.subscription_data,
            task_subscribe: None,
        }
    }
}

impl<T: Message + 'static> Node<Idle, T> {
    /// Attempt connection from the Node to the Host located at the specified address
    #[tracing::instrument]
    pub fn activate(mut self) -> Result<Node<Active, T>, Box<dyn Error>> {
        // let ip = crate::get_ip(interface).unwrap();
        // dbg!(ip);
        let addr = self.host_addr_tcp;
        let topic = self.topic.clone();

        let stream = self.runtime.block_on(async move {
            let stream = try_connection(addr).await.unwrap();
            let stream = handshake(stream, topic).await.unwrap();
            stream
        });
        self.stream = Some(stream);

        Ok(Node::<Active, T>::from(self))
    }

    #[tracing::instrument]
    pub fn subscribe(mut self, rate: Duration) -> Result<Node<Subscription, T>, Box<dyn Error>> {
        let name = self.name.clone() + "_SUBSCRIPTION";
        let addr = self.host_addr_tcp;
        let topic = self.topic.clone();

        let subscription_data: Arc<TokioMutex<Option<SubscriptionData<T>>>> =
            Arc::new(TokioMutex::new(None));
        let data = Arc::clone(&subscription_data);

        let task_subscribe = self.runtime.spawn(async move {
            //let mut subscription_node = subscription_node.connect().unwrap();
            let stream = try_connection(addr).await.unwrap();
            let stream = handshake(stream, topic.clone()).await.unwrap();
            let packet = GenericMsg {
                msg_type: MsgType::GET,
                timestamp: Utc::now().to_string(),
                name: name.clone(),
                topic: topic.clone(),
                data_type: std::any::type_name::<T>().to_string(),
                data: Vec::new(),
            };
            // info!("{:?}",&packet);

            loop {
                let packet_as_bytes: Vec<u8> = to_allocvec(&packet).unwrap();
                send_msg(&mut &stream, packet_as_bytes).await.unwrap();
                let reply = match await_response::<T>(&mut &stream, 4096).await {
                    Ok(val) => val,
                    Err(e) => {
                        error!("subscription error: {}", e);
                        continue;
                    }
                };
                let delta = Utc::now() - reply.timestamp.parse::<DateTime<Utc>>().unwrap();
                // println!("The time difference between msg tx/rx is: {} us",delta);
                if delta <= chrono::Duration::zero() {
                    // println!("Data is not newer, skipping to next subscription iteration");
                    continue;
                }
                // info!("Node has received msg data: {:?}",&msg.data);
                let reply_data = match from_bytes::<T>(&reply.data) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("{:?}", e);
                        continue;
                    }
                };
                let reply_sub_data = SubscriptionData {
                    data: reply_data,
                    timestamp: reply.timestamp,
                };
                let mut data = data.lock().await;

                *data = Some(reply_sub_data);
                sleep(rate).await;
            }
        });
        self.task_subscribe = Some(task_subscribe);
        println!("spawned subscription task");

        let mut subscription_node = Node::<Subscription, T>::from(self);
        subscription_node.subscription_data = subscription_data;

        Ok(subscription_node)
    }
}
