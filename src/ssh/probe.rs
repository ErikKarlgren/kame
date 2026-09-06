// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use russh::client::{self, Handler};

pub struct SshProber {
    config: Arc<client::Config>,
}

pub struct ShhProbeResult {
    pub latency: Duration,
}

impl SshProber {
    pub fn new(timeout: Duration) -> Self {
        let config = client::Config {
            inactivity_timeout: Some(timeout),
            ..<_>::default()
        };
        let config = Arc::new(config);
        Self { config }
    }

    pub async fn connect(&self, hostname: &str, port: u32) -> Result<ShhProbeResult> {
        let addrs = format!("{hostname}:{port}");
        let handler = Client {};

        let start = Instant::now();

        _ = client::connect(self.config.clone(), &addrs, handler)
            .await
            .with_context(|| format!("Error while trying to connect to {addrs}"))?;

        let elapsed = Instant::now().duration_since(start);
        Ok(ShhProbeResult { latency: elapsed })
    }
}

struct Client {}

impl Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        Ok(false) // We don't want to connect
    }
}
