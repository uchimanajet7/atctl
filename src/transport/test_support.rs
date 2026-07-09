use std::collections::VecDeque;
use std::time::Duration;

use crate::transport::traits::AtTransport;
use crate::{AtctlError, Result};

#[derive(Debug, Default)]
pub(crate) struct MockTransport {
    opened: bool,
    written: Vec<String>,
    responses: VecDeque<Vec<u8>>,
}

impl MockTransport {
    pub(crate) fn with_response(response: Vec<u8>) -> Self {
        let mut responses = VecDeque::new();
        responses.push_back(response);
        Self {
            opened: true,
            written: Vec::new(),
            responses,
        }
    }

    pub(crate) fn with_responses<I>(responses: I) -> Self
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        Self {
            opened: true,
            written: Vec::new(),
            responses: responses.into_iter().collect(),
        }
    }

    pub(crate) fn written_commands(&self) -> &[String] {
        &self.written
    }
}

impl AtTransport for MockTransport {
    fn open(&mut self) -> Result<()> {
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.opened = false;
        Ok(())
    }

    fn write_command(&mut self, command: &str) -> Result<()> {
        if !self.opened {
            return Err(AtctlError::Transport("mock transport is closed".into()));
        }
        self.written.push(command.to_string());
        Ok(())
    }

    fn read_response(&mut self, _timeout: Duration) -> Result<Vec<u8>> {
        if !self.opened {
            return Err(AtctlError::Transport("mock transport is closed".into()));
        }
        self.responses.pop_front().ok_or(AtctlError::Timeout)
    }
}
