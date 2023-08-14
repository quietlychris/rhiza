use crate::Error;
use crate::{Message, Quic, Subscription};

impl<T: Message + 'static> Node<Quic, Subscription, T> {
    // Should actually return a <T>
    pub fn get_subscribed_data(&self) -> Result<T, crate::Error> {
        let data = self.subscription_data.clone();
        self.runtime.block_on(async {
            let data = data.lock().await;
            match data.clone() {
                Some(value) => Ok(value.data),
                None => Err(Error::NoSubscriptionValue),
            }
        })
    }
}
