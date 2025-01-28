use crate::error::HostError;
use crate::error::HostOperation;
use crate::Error;
use chrono::{DateTime, Utc};
use postcard::to_allocvec;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::convert::{Into, TryFrom, TryInto};

use std::fmt::Debug;
/// Trait for Meadow-compatible data, requiring serde De\Serialize, Debug, and Clone
pub trait Message: Serialize + DeserializeOwned + Debug + Sync + Send + Clone {}
impl<T> Message for T where T: Serialize + DeserializeOwned + Debug + Sync + Send + Clone {}

/// Msg definitions for publish or request of topic data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(C)]
pub enum MsgType {
    /// Request `Set` operation on Host
    Set,
    /// Request `Get` operation on Host
    Get,
    /// Request `GetNth` operation on Host
    GetNth(usize),
    /// Request list of topics from Host  
    Topics,
    /// Request start of subscribe operation from Host
    Subscribe,
    /// Communicate success or failure of certain Host-side operations
    HostOperation(HostOperation),
}

/// Message format containing a strongly-typed data payload and associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct Msg<T> {
    /// Type of `meadow` message
    pub msg_type: MsgType,
    /// Message timestamp in Utc
    pub timestamp: DateTime<Utc>,
    /// Topic name
    pub topic: String,
    /// Name of message's data type (`String`-typed)
    pub data_type: String,
    /// Strongly-typed data payload
    pub data: T,
}

impl<T: Message> Msg<T> {
    /// Create a new strongly-typed message (default timestamp is from `SystemTime` in UTC)
    pub fn new(msg_type: MsgType, topic: impl Into<String>, data: T) -> Self {
        Msg {
            msg_type,
            timestamp: Utc::now(),
            topic: topic.into(),
            data_type: std::any::type_name::<T>().to_string(),
            data,
        }
    }

    /// Set the message's topic
    pub fn set_topic(&mut self, topic: impl Into<String>) {
        self.topic = topic.into();
    }

    /// Set the message's timestamp
    pub fn set_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.timestamp = timestamp;
    }

    /// Set the message's data payload
    pub fn set_data(&mut self, data: T) {
        self.data = data;
    }

    /// Attempt conversion to `GenericMsg`
    pub fn to_generic(self) -> Result<GenericMsg, crate::Error> {
        self.try_into()
    }
}

/// Message format containing a generic `Vec<u8>` data payload and associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct GenericMsg {
    /// Type of `meadow` message
    pub msg_type: MsgType,
    /// Message timestamp in Utc
    pub timestamp: DateTime<Utc>,
    /// Topic name
    pub topic: String,
    /// Name of message's data type (`String`-typed)
    pub data_type: String,
    /// Generic byte-represented data payload
    pub data: Vec<u8>,
}

impl GenericMsg {
    /// Create a default `MsgType::Set` message for published messages
    #[inline]
    pub fn set<T: Message>(topic: impl Into<String>, data: Vec<u8>) -> Self {
        GenericMsg {
            msg_type: MsgType::Set,
            timestamp: Utc::now(),
            topic: topic.into(),
            data_type: std::any::type_name::<T>().to_string(),
            data,
        }
    }

    /// Create a default `MsgType::Get` message for requests
    #[inline]
    pub fn get<T: Message>(topic: impl Into<String>) -> Self {
        GenericMsg {
            msg_type: MsgType::Get,
            timestamp: Utc::now(),
            topic: topic.into(),
            data_type: std::any::type_name::<T>().to_string(),
            data: Vec::new(),
        }
    }

    /// Create a `MsgType::GetNth` message for requests
    #[inline]
    pub fn get_nth<T: Message>(topic: impl Into<String>, n: usize) -> Self {
        GenericMsg {
            msg_type: MsgType::GetNth(n),
            timestamp: Utc::now(),
            topic: topic.into(),
            data_type: std::any::type_name::<T>().to_string(),
            data: Vec::new(),
        }
    }

    /// Create a default `MsgType::Topics` message
    #[inline]
    pub fn topics() -> Self {
        GenericMsg {
            msg_type: MsgType::Topics,
            timestamp: Utc::now(),
            topic: String::new(),
            data_type: std::any::type_name::<()>().to_string(),
            data: Vec::new(),
        }
    }

    /// Create a generic
    pub fn host_operation(op: HostOperation) -> Self {
        GenericMsg {
            msg_type: MsgType::HostOperation(op),
            timestamp: Utc::now(),
            topic: String::new(),
            data_type: std::any::type_name::<()>().to_string(),
            data: Vec::new(),
        }
    }

    /// Convert `GenericMsg` into a `postcard`-encoded byte string
    pub fn as_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(&self)
    }
}

impl<T: Message> TryInto<Msg<T>> for GenericMsg {
    type Error = crate::Error;

    fn try_into(self) -> Result<Msg<T>, Error> {
        let data = postcard::from_bytes::<T>(&self.data[..])?;
        Ok(Msg {
            msg_type: self.msg_type,
            timestamp: self.timestamp,
            topic: self.topic.clone(),
            data_type: self.data_type.clone(),
            data,
        })
    }
}

impl<T: Message> TryInto<GenericMsg> for Msg<T> {
    type Error = crate::Error;

    fn try_into(self) -> Result<GenericMsg, Error> {
        let data = postcard::to_allocvec(&self.data)?;
        Ok(GenericMsg {
            msg_type: self.msg_type,
            timestamp: self.timestamp,
            topic: self.topic.clone(),
            data_type: self.data_type.clone(),
            data,
        })
    }
}
