use std::time::Duration;

use reqwest::{Client, redirect};

use crate::Result;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REDIRECT_LIMIT: usize = 5;

pub(crate) fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(redirect::Policy::limited(REDIRECT_LIMIT))
        .build()?)
}
