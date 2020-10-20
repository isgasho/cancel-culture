use fantoccini::{error::CmdError, Client};
use futures::{future::BoxFuture, TryFutureExt};
use std::collections::HashSet;
use std::hash::Hash;
use std::time::Duration;
use tokio::time::delay_for;

/// Represents a page of stuff that can be scrolled through and extracted
pub trait Scroller {
    type Item;
    type Err: From<CmdError> + Send + 'static;

    fn init<'a>(&'a self, client: &'a mut Client) -> BoxFuture<'a, Result<(), Self::Err>>;
    fn extract<'a>(
        &'a self,
        client: &'a mut Client,
    ) -> BoxFuture<'a, Result<Vec<Self::Item>, Self::Err>>;

    fn advance(client: &mut Client) -> BoxFuture<Result<(), Self::Err>> {
        Box::pin(async move {
            let mut element = client.active_element().await?;
            element.send_keys(" ").err_into().await
        })
    }

    fn wait() -> Option<Duration> {
        Some(Duration::from_millis(500))
    }

    fn max_attempts() -> usize {
        5
    }

    fn extract_all<'a>(
        &'a self,
        client: &'a mut Client,
    ) -> BoxFuture<'a, Result<Vec<Self::Item>, Self::Err>>
    where
        Self::Item: Clone + Eq + Hash + Send,
        Self: Sized + Send + Sync,
    {
        Box::pin(async move {
            self.init(client).await?;
            if let Some(duration) = Self::wait() {
                delay_for(duration).await;
            }

            let mut result = self.extract(client).await?;
            let mut seen = HashSet::new();
            seen.extend(result.iter().cloned());

            let mut remaining = Self::max_attempts();

            while remaining > 0 {
                Self::advance(client).await?;
                if let Some(duration) = Self::wait() {
                    delay_for(duration).await;
                }

                let batch = self.extract(client).await?;
                let mut empty = true;

                for item in batch.into_iter() {
                    if !seen.contains(&item) {
                        empty = false;
                        seen.insert(item.clone());
                        result.push(item);
                    }
                }

                if empty {
                    remaining -= 1;
                } else {
                    remaining = Self::max_attempts();
                }
            }

            Ok(result)
        })
    }
}
