use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::app::operation::ExecutionSurface;
use crate::app::operation::OperationIdentity;
use crate::at::command::{command_with_terminator, has_embedded_command_framing};
use crate::at::mask::{mask_declared_sensitive_value, mask_sensitive_values};
use crate::at::parser::parse_response;
use crate::at::response::AtStatus;
use crate::at::review::{Assessment, ExecutionReview, review_direct_command};
use crate::at::review::{is_prompt_required_command, is_supported_bridge_prompt_command};
use crate::log::raw::{RawLogConfig, RawLogExchange, RawLogSink, RawLogTransportError};
use crate::transport::traits::{AtTransport, ResponseMatcher};
use crate::transport::usb::{UsbAtTransport, UsbAtTransportConfig};
use crate::{AtctlError, Result};

#[derive(Debug)]
pub struct PtyBridgeConfig {
    pub symlink: PathBuf,
    pub replace_symlink: bool,
    pub raw_log: Option<RawLogConfig>,
    pub usb: UsbAtTransportConfig,
    pub command_timeout: Duration,
}

#[cfg(unix)]
pub fn run_usb_bridge(config: PtyBridgeConfig) -> Result<()> {
    use portable_pty::{PtySize, native_pty_system};

    let mut transport = UsbAtTransport::new(config.usb);
    transport.open()?;
    let mut raw_log = config.raw_log.map(RawLogSink::create).transpose()?;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize::default())
        .map_err(|error| AtctlError::Transport(format!("failed to open PTY: {error}")))?;
    let slave_path = pair
        .master
        .tty_name()
        .ok_or_else(|| AtctlError::Transport("PTY slave path is unavailable".to_owned()))?;

    let stop = Arc::new(AtomicBool::new(true));
    install_signal_handler(stop.clone())?;

    let _symlink = SymlinkGuard::create(&config.symlink, &slave_path, config.replace_symlink)?;

    println!("PTY bridge ready");
    println!("symlink: {}", config.symlink.display());
    println!("target: {}", slave_path.display());
    println!("connect: screen {} 115200", config.symlink.display());
    println!("note: 115200 is a serial-tool compatibility value, not physical UART speed");
    if let Some(raw_log) = raw_log.as_ref() {
        println!("raw log: {}", raw_log.path().display());
    }
    println!("press Ctrl-C to stop");

    run_bridge_loop(
        pair,
        &mut transport,
        config.command_timeout,
        raw_log.as_mut(),
        stop,
    )?;
    transport.close()
}

#[cfg(not(unix))]
pub fn run_usb_bridge(_config: PtyBridgeConfig) -> Result<()> {
    Err(AtctlError::NotImplemented(
        "PTY bridge is only implemented for Unix-like platforms",
    ))
}

#[cfg(unix)]
fn install_signal_handler(stop: Arc<AtomicBool>) -> Result<()> {
    ctrlc::set_handler(move || {
        stop.store(false, Ordering::SeqCst);
    })
    .map_err(|error| AtctlError::Transport(format!("failed to install signal handler: {error}")))
}

#[cfg(unix)]
fn run_bridge_loop<T>(
    pair: portable_pty::PtyPair,
    transport: &mut T,
    command_timeout: Duration,
    mut raw_log: Option<&mut RawLogSink>,
    stop: Arc<AtomicBool>,
) -> Result<()>
where
    T: AtTransport,
{
    let portable_pty::PtyPair { master, slave } = pair;
    let reader = master
        .try_clone_reader()
        .map_err(|error| AtctlError::Transport(format!("failed to clone PTY reader: {error}")))?;
    let mut writer = master
        .take_writer()
        .map_err(|error| AtctlError::Transport(format!("failed to take PTY writer: {error}")))?;
    let (sender, receiver) = mpsc::channel();
    let _reader_thread = thread::spawn(move || read_pty_input(reader, sender));
    let _slave = slave;
    let _master = master;
    let mut state = BridgeCommandState::default();

    while stop.load(Ordering::SeqCst) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(PtyInput::Line(line)) => {
                if handle_bridge_line(
                    &mut state,
                    transport,
                    &mut writer,
                    line,
                    command_timeout,
                    raw_log.as_deref_mut(),
                )? == BridgeLoopAction::Stop
                {
                    break;
                }
            }
            Ok(PtyInput::Eof) => break,
            Ok(PtyInput::ReadError(error)) => {
                return Err(AtctlError::Transport(format!(
                    "failed to read PTY: {error}"
                )));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn read_pty_input(mut reader: Box<dyn Read + Send>, sender: mpsc::Sender<PtyInput>) {
    let mut decoder = PtyLineDecoder::default();
    let mut buffer = [0_u8; 256];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                if let Some(line) = decoder.flush() {
                    let _ = sender.send(PtyInput::Line(line));
                }
                let _ = sender.send(PtyInput::Eof);
                break;
            }
            Ok(read) => {
                for line in decoder.push(&buffer[..read]) {
                    if sender.send(PtyInput::Line(line)).is_err() {
                        return;
                    }
                }
            }
            Err(error) if is_pty_client_disconnect(&error) => {
                let _ = sender.send(PtyInput::Eof);
                break;
            }
            Err(error) => {
                let _ = sender.send(PtyInput::ReadError(error.to_string()));
                break;
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PtyInput {
    Line(String),
    Eof,
    ReadError(String),
}

#[derive(Debug, Default)]
struct PtyLineDecoder {
    buffer: Vec<u8>,
}

impl PtyLineDecoder {
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for byte in bytes {
            match byte {
                b'\r' | b'\n' => {
                    if let Some(line) = self.finish_line() {
                        lines.push(line);
                    }
                }
                _ => self.buffer.push(*byte),
            }
        }
        lines
    }

    fn flush(&mut self) -> Option<String> {
        self.finish_line()
    }

    fn finish_line(&mut self) -> Option<String> {
        let line = String::from_utf8_lossy(&self.buffer).trim().to_owned();
        self.buffer.clear();
        (!line.is_empty()).then_some(line)
    }
}

#[derive(Debug, Default)]
struct BridgeCommandState {
    pending_confirmation: Option<PendingConfirmation>,
    pending_payload: Option<PendingPayload>,
}

#[derive(Debug)]
struct PendingConfirmation {
    command: String,
    payload: Option<String>,
    review: ExecutionReview,
}

#[derive(Debug)]
struct PendingPayload {
    command: String,
    review: ExecutionReview,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum BridgeLoopAction {
    Continue,
    Stop,
}

fn handle_bridge_line<T, W>(
    state: &mut BridgeCommandState,
    transport: &mut T,
    writer: &mut W,
    line: String,
    command_timeout: Duration,
    raw_log: Option<&mut RawLogSink>,
) -> Result<BridgeLoopAction>
where
    T: AtTransport,
    W: Write,
{
    if let Some(pending) = state.pending_payload.take() {
        if write_confirmation_prompt(writer, &pending.review, Some(&line))?
            == BridgeLoopAction::Stop
        {
            return Ok(BridgeLoopAction::Stop);
        }
        state.pending_confirmation = Some(PendingConfirmation {
            command: pending.command,
            payload: Some(line),
            review: pending.review,
        });
        return Ok(BridgeLoopAction::Continue);
    }

    if let Some(pending) = state.pending_confirmation.take() {
        if line.trim() == confirmation_token(&pending.review) {
            return execute_bridge_command_or_prompt(
                transport,
                writer,
                &pending.command,
                pending.payload.as_deref(),
                command_timeout,
                &pending.review,
                raw_log,
            );
        }

        return write_pty_line(writer, "atctl: operation cancelled; nothing sent\r\n");
    }

    if has_embedded_command_framing(&line) {
        return write_pty_line(
            writer,
            "atctl: operation rejected; embedded command framing or control characters are not allowed\r\n",
        );
    }
    let review = review_direct_command(&line);
    if is_prompt_required_command(&line) {
        if !is_supported_bridge_prompt_command(&line) {
            return write_pty_line(
                writer,
                "atctl: operation rejected; this prompt/payload command requires a Sequence so its payload framing can be handled correctly.\r\n",
            );
        }
        state.pending_payload = Some(PendingPayload {
            command: line,
            review,
        });
        return write_pty_line(
            writer,
            "atctl: enter the complete payload line; command and payload will be reviewed before anything is sent.\r\n",
        );
    }
    if review.requires_confirmation() {
        if write_confirmation_prompt(writer, &review, None)? == BridgeLoopAction::Stop {
            return Ok(BridgeLoopAction::Stop);
        }
        state.pending_confirmation = Some(PendingConfirmation {
            command: line,
            payload: None,
            review,
        });
        return Ok(BridgeLoopAction::Continue);
    }

    execute_bridge_command_or_prompt(
        transport,
        writer,
        &line,
        None,
        command_timeout,
        &review,
        raw_log,
    )
}

fn execute_bridge_command_or_prompt<T, W>(
    transport: &mut T,
    writer: &mut W,
    command: &str,
    payload: Option<&str>,
    command_timeout: Duration,
    review: &ExecutionReview,
    raw_log: Option<&mut RawLogSink>,
) -> Result<BridgeLoopAction>
where
    T: AtTransport,
    W: Write,
{
    if is_prompt_required_command(command) {
        let payload = payload.ok_or_else(|| AtctlError::InvalidValue {
            name: "PTY payload",
            value: "missing complete payload before execution".to_owned(),
        })?;
        let mut raw_log = raw_log;
        let action = execute_bridge_prompt_command(
            transport,
            writer,
            command,
            command_timeout,
            review,
            raw_log.as_deref_mut(),
        )?;
        if action == BridgeLoopAction::Stop {
            return Ok(action);
        }
        return execute_bridge_payload(
            transport,
            writer,
            command,
            payload,
            command_timeout,
            review,
            raw_log,
        );
    }

    execute_bridge_command(transport, writer, command, command_timeout, review, raw_log)
}

fn write_confirmation_prompt<W>(
    writer: &mut W,
    review: &ExecutionReview,
    payload: Option<&str>,
) -> Result<BridgeLoopAction>
where
    W: Write,
{
    fn write_prompt_line<W>(writer: &mut W, text: &str) -> Result<bool>
    where
        W: Write,
    {
        Ok(write_pty_line(writer, text)? == BridgeLoopAction::Continue)
    }

    if !write_prompt_line(
        writer,
        "\r\nCommand requires confirmation before sending.\r\n",
    )? {
        return Ok(BridgeLoopAction::Stop);
    }
    if !write_prompt_line(
        writer,
        &format!(
            "AT command: {}\r\n",
            mask_sensitive_values(&review.normalized_operation)
        ),
    )? {
        return Ok(BridgeLoopAction::Stop);
    }
    if let Some(payload) = payload
        && !write_prompt_line(
            writer,
            &format!("Payload: {}\r\n", mask_declared_sensitive_value(payload)),
        )?
    {
        return Ok(BridgeLoopAction::Stop);
    }
    if review.assessment == Assessment::Unknown
        && !write_prompt_line(writer, "! Effects are not fully known\r\n")?
    {
        return Ok(BridgeLoopAction::Stop);
    }
    for section in review.presentation_sections() {
        if !write_prompt_line(writer, &format!("{}:\r\n", section.kind.heading()))? {
            return Ok(BridgeLoopAction::Stop);
        }
        for item in section.items {
            if !write_prompt_line(writer, &format!("  - {item}\r\n"))? {
                return Ok(BridgeLoopAction::Stop);
            }
        }
    }
    write_pty_line(
        writer,
        &format!(
            "Type `{}` to continue, or send any other line to cancel.\r\n",
            confirmation_token(review)
        ),
    )
}

fn execute_bridge_command<T, W>(
    transport: &mut T,
    writer: &mut W,
    command: &str,
    command_timeout: Duration,
    review: &ExecutionReview,
    raw_log: Option<&mut RawLogSink>,
) -> Result<BridgeLoopAction>
where
    T: AtTransport,
    W: Write,
{
    let started = Instant::now();
    ensure_current_review(review, command)?;
    let tx = command_with_terminator(command);
    if let Err(error) = transport.write_command(&tx) {
        append_bridge_raw_error(
            raw_log,
            command,
            review,
            started.elapsed(),
            "write_command",
            &error,
            tx.as_bytes(),
        )?;
        write_pty_line(
            writer,
            &format!("atctl: bridge stopping after transport error: {error}\r\n"),
        )?;
        return Err(error);
    }

    let raw_response = match transport.read_response(command_timeout) {
        Ok(raw_response) => raw_response,
        Err(error) => {
            append_bridge_raw_error(
                raw_log,
                command,
                review,
                started.elapsed(),
                "read_response",
                &error,
                tx.as_bytes(),
            )?;
            write_pty_line(
                writer,
                &format!("atctl: bridge stopping after transport error: {error}\r\n"),
            )?;
            return Err(error);
        }
    };

    let response = parse_response(&raw_response);
    let duration = started.elapsed();
    if let Some(raw_log) = raw_log {
        let operation = OperationIdentity::command(command);
        raw_log.append_exchange(RawLogExchange {
            operation: &operation,
            command_name: None,
            command,
            review,
            status: &response.status,
            duration,
            tx_bytes: tx.as_bytes(),
            rx_bytes: &response.raw,
        })?;
    }
    write_response_text(writer, &mask_sensitive_values(&response.text))
}

fn execute_bridge_prompt_command<T, W>(
    transport: &mut T,
    writer: &mut W,
    command: &str,
    command_timeout: Duration,
    review: &ExecutionReview,
    raw_log: Option<&mut RawLogSink>,
) -> Result<BridgeLoopAction>
where
    T: AtTransport,
    W: Write,
{
    let started = Instant::now();
    ensure_current_review(review, command)?;
    let tx = command_with_terminator(command);
    if let Err(error) = transport.write_command(&tx) {
        append_bridge_raw_error(
            raw_log,
            command,
            review,
            started.elapsed(),
            "write_command",
            &error,
            tx.as_bytes(),
        )?;
        write_pty_line(
            writer,
            &format!("atctl: bridge stopping after transport error: {error}\r\n"),
        )?;
        return Err(error);
    }

    let raw_response =
        match transport.read_until(command_timeout, ResponseMatcher::Contains(">".to_owned())) {
            Ok(raw_response) => raw_response,
            Err(error) => {
                append_bridge_raw_error(
                    raw_log,
                    command,
                    review,
                    started.elapsed(),
                    "read_prompt",
                    &error,
                    tx.as_bytes(),
                )?;
                write_pty_line(
                    writer,
                    &format!("atctl: bridge stopping after transport error: {error}\r\n"),
                )?;
                return Err(error);
            }
        };

    let response = parse_response(&raw_response);
    let duration = started.elapsed();
    if let Some(raw_log) = raw_log {
        let operation = OperationIdentity::command(command);
        raw_log.append_exchange(RawLogExchange {
            operation: &operation,
            command_name: None,
            command,
            review,
            status: &AtStatus::Ok,
            duration,
            tx_bytes: tx.as_bytes(),
            rx_bytes: &response.raw,
        })?;
    }
    if write_response_text(writer, &mask_sensitive_values(&response.text))?
        == BridgeLoopAction::Stop
    {
        return Ok(BridgeLoopAction::Stop);
    }
    Ok(BridgeLoopAction::Continue)
}

fn execute_bridge_payload<T, W>(
    transport: &mut T,
    writer: &mut W,
    command: &str,
    payload: &str,
    command_timeout: Duration,
    review: &ExecutionReview,
    raw_log: Option<&mut RawLogSink>,
) -> Result<BridgeLoopAction>
where
    T: AtTransport,
    W: Write,
{
    let started = Instant::now();
    let tx = format!("{payload}\u{1a}");
    if let Err(error) = transport.write_command(&tx) {
        append_bridge_raw_error(
            raw_log,
            command,
            review,
            started.elapsed(),
            "write_payload",
            &error,
            tx.as_bytes(),
        )?;
        write_pty_line(
            writer,
            &format!("atctl: bridge stopping after transport error: {error}\r\n"),
        )?;
        return Err(error);
    }

    let raw_response = match transport.read_response(command_timeout) {
        Ok(raw_response) => raw_response,
        Err(error) => {
            append_bridge_raw_error(
                raw_log,
                command,
                review,
                started.elapsed(),
                "read_payload_response",
                &error,
                tx.as_bytes(),
            )?;
            write_pty_line(
                writer,
                &format!("atctl: bridge stopping after transport error: {error}\r\n"),
            )?;
            return Err(error);
        }
    };

    let response = parse_response(&raw_response);
    let duration = started.elapsed();
    if let Some(raw_log) = raw_log {
        let operation = OperationIdentity::command(command);
        raw_log.append_exchange(RawLogExchange {
            operation: &operation,
            command_name: None,
            command,
            review,
            status: &response.status,
            duration,
            tx_bytes: tx.as_bytes(),
            rx_bytes: &response.raw,
        })?;
    }
    let masked = mask_bridge_payload_response(&response.text, payload);
    write_response_text(writer, &masked)
}

fn mask_bridge_payload_response(text: &str, payload: &str) -> String {
    let masked = mask_sensitive_values(text);
    if payload.is_empty() {
        masked
    } else {
        masked.replace(payload, &mask_declared_sensitive_value(payload))
    }
}

fn append_bridge_raw_error(
    raw_log: Option<&mut RawLogSink>,
    command: &str,
    review: &ExecutionReview,
    duration: Duration,
    stage: &'static str,
    error: &AtctlError,
    tx_bytes: &[u8],
) -> Result<()> {
    let Some(raw_log) = raw_log else {
        return Ok(());
    };
    let error = error.to_string();
    let operation = OperationIdentity::command(command);
    raw_log.append_transport_error(RawLogTransportError {
        operation: &operation,
        command_name: None,
        command,
        review,
        duration,
        stage,
        error: &error,
        tx_bytes,
        rx_bytes: b"",
    })
}

fn confirmation_token(review: &ExecutionReview) -> &'static str {
    if review.assessment == Assessment::Unknown {
        "unknown"
    } else {
        "run"
    }
}

fn ensure_current_review(review: &ExecutionReview, command: &str) -> Result<()> {
    let current = review_direct_command(command);
    if &current != review {
        return Err(AtctlError::OperationOutsideReviewedBoundary {
            operation: current.normalized_operation,
        });
    }
    Ok(())
}

fn write_response_text<W>(writer: &mut W, text: &str) -> Result<BridgeLoopAction>
where
    W: Write,
{
    if write_pty_line(writer, text)? == BridgeLoopAction::Stop {
        return Ok(BridgeLoopAction::Stop);
    }
    if !text.ends_with('\n') && !text.ends_with('\r') {
        return write_pty_line(writer, "\r\n");
    }
    Ok(BridgeLoopAction::Continue)
}

fn write_pty_line<W>(writer: &mut W, text: &str) -> Result<BridgeLoopAction>
where
    W: Write,
{
    if let Err(error) = writer.write_all(text.as_bytes()) {
        if is_pty_client_disconnect(&error) {
            return Ok(BridgeLoopAction::Stop);
        }
        return Err(AtctlError::Transport(format!(
            "failed to write PTY: {error}"
        )));
    }
    if let Err(error) = writer.flush() {
        if is_pty_client_disconnect(&error) {
            return Ok(BridgeLoopAction::Stop);
        }
        return Err(AtctlError::Transport(format!(
            "failed to flush PTY: {error}"
        )));
    }
    Ok(BridgeLoopAction::Continue)
}

fn is_pty_client_disconnect(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset | ErrorKind::NotConnected
    ) || error.raw_os_error() == Some(5)
}

#[cfg(unix)]
#[derive(Debug)]
struct SymlinkGuard {
    link: PathBuf,
    target: PathBuf,
}

#[cfg(unix)]
impl SymlinkGuard {
    fn create(link: &PathBuf, target: &PathBuf, replace_symlink: bool) -> Result<Self> {
        match std::fs::symlink_metadata(link) {
            Ok(metadata) => {
                if !metadata.file_type().is_symlink() {
                    return Err(AtctlError::Transport(format!(
                        "refusing to overwrite non-symlink path: {}",
                        link.display()
                    )));
                }
                if !replace_symlink {
                    return Err(AtctlError::Transport(format!(
                        "symlink already exists: {}; use --replace-symlink to replace it",
                        link.display()
                    )));
                }
                std::fs::remove_file(link).map_err(|error| {
                    AtctlError::Transport(format!(
                        "failed to remove existing symlink {}: {error}",
                        link.display()
                    ))
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AtctlError::Transport(format!(
                    "failed to inspect symlink path {}: {error}",
                    link.display()
                )));
            }
        }

        std::os::unix::fs::symlink(target, link).map_err(|error| {
            AtctlError::Transport(format!(
                "failed to create symlink {} -> {}: {error}",
                link.display(),
                target.display()
            ))
        })?;

        Ok(Self {
            link: link.clone(),
            target: target.clone(),
        })
    }

    fn cleanup(&self) -> Result<()> {
        match std::fs::read_link(&self.link) {
            Ok(current_target) if current_target == self.target => {
                std::fs::remove_file(&self.link).map_err(|error| {
                    AtctlError::Transport(format!(
                        "failed to remove symlink {}: {error}",
                        self.link.display()
                    ))
                })?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AtctlError::Transport(format!(
                    "failed to inspect symlink {} during cleanup: {error}",
                    self.link.display()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for SymlinkGuard {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use std::io::{
        Error as IoError, ErrorKind as IoErrorKind, Result as IoResult, Write as IoWrite,
    };
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::transport::test_support::MockTransport;

    use super::*;

    #[test]
    fn line_decoder_splits_cr_lf_and_ignores_empty_lines() {
        let mut decoder = PtyLineDecoder::default();

        let lines = decoder.push(b"AT\r\n\nAT+CIMI\rpartial");
        assert_eq!(lines, ["AT", "AT+CIMI"]);
        assert_eq!(decoder.flush(), Some("partial".to_owned()));
    }

    #[test]
    fn known_observation_executes_and_masks_response() {
        let mut state = BridgeCommandState::default();
        let mut transport =
            MockTransport::with_response(b"AT+CIMI\r\r\n295050912389644\r\n\r\nOK\r\n".to_vec());
        let mut output = Vec::new();

        let action = handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+CIMI".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert_eq!(action, BridgeLoopAction::Continue);
        assert_eq!(transport.written_commands(), ["AT+CIMI\r"]);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("29505091******"));
        assert!(!output.contains("295050912389644"));
    }

    #[test]
    fn known_short_phone_field_is_masked_without_hiding_operational_values() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(
            b"AT+CNUM\r\r\n+CNUM: \"line\",\"901001\",129\r\n+CMGS: 7\r\nOK\r\n".to_vec(),
        );
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+CNUM".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "unknown".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\"******\",129"));
        assert!(output.contains("+CMGS: 7"));
        assert!(!output.contains("901001"));
    }

    #[test]
    fn pty_client_disconnect_during_response_stops_without_transport_error() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"AT\r\r\nOK\r\n".to_vec());
        let mut writer = BrokenPipeWriter;

        let action = handle_bridge_line(
            &mut state,
            &mut transport,
            &mut writer,
            "AT".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert_eq!(action, BridgeLoopAction::Stop);
        assert_eq!(transport.written_commands(), ["AT\r"]);
    }

    #[test]
    fn pty_client_disconnect_during_confirmation_prompt_does_not_leave_pending_command() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"ATE0\r\r\nOK\r\n".to_vec());
        let mut writer = BrokenPipeWriter;

        let action = handle_bridge_line(
            &mut state,
            &mut transport,
            &mut writer,
            "ATE0".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert_eq!(action, BridgeLoopAction::Stop);
        assert!(state.pending_confirmation.is_none());
        assert!(transport.written_commands().is_empty());
    }

    #[test]
    fn pty_client_disconnect_during_flush_stops_without_transport_error() {
        let mut writer = FlushBrokenPipeWriter;

        let action = write_pty_line(&mut writer, "OK\r\n").unwrap();

        assert_eq!(action, BridgeLoopAction::Stop);
    }

    #[test]
    fn pty_eio_is_treated_as_client_disconnect() {
        let error = IoError::from_raw_os_error(5);

        assert!(is_pty_client_disconnect(&error));
    }

    #[test]
    fn known_confirmation_required_command_waits_for_run_token() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"ATE0\r\r\nOK\r\n".to_vec());
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "ATE0".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        let prompt = String::from_utf8(output.clone()).unwrap();
        assert!(prompt.contains("AT command: ATE0"));
        assert!(prompt.contains("What will happen:"));
        assert!(!prompt.contains("Assessment:"));
        assert!(!prompt.contains("Confirmation: required"));
        assert!(prompt.contains("Type `run`"));
        assert!(!prompt.contains("Risk:"));
        assert!(transport.written_commands().is_empty());

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "run".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert_eq!(transport.written_commands(), ["ATE0\r"]);
    }

    #[test]
    fn wrong_confirmation_cancels_without_sending() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"ATE0\r\r\nOK\r\n".to_vec());
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "ATE0".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "abc".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert!(transport.written_commands().is_empty());
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("operation cancelled")
        );
    }

    #[test]
    fn unknown_confirmation_names_uncertainty_before_unknown_token() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+VENDOR=1".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        let prompt = String::from_utf8(output).unwrap();
        assert!(prompt.contains("AT command: AT+VENDOR=1"));
        assert!(prompt.contains("! Effects are not fully known"));
        assert!(prompt.contains("Known:"));
        assert!(prompt.contains("Possible consequences:"));
        assert!(prompt.contains("Not established:"));
        assert!(prompt.contains("Type `unknown`"));
        assert!(!prompt.contains("Assessment:"));
        assert!(transport.written_commands().is_empty());
    }

    #[test]
    fn prompt_required_command_waits_for_payload_line_and_appends_ctrl_z() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_responses([
            b"\r\n> ".to_vec(),
            b"\r\n+CMGS: 12\r\n\r\nOK\r\n".to_vec(),
        ]);
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+CMGS=\"+819012345678\"".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        assert!(state.pending_confirmation.is_none());
        assert!(state.pending_payload.is_some());
        assert!(transport.written_commands().is_empty());

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "hello from atctl".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        assert!(state.pending_confirmation.is_some());
        assert!(state.pending_payload.is_none());
        assert!(transport.written_commands().is_empty());
        let prompt_output = String::from_utf8(output.clone()).unwrap();
        assert!(prompt_output.contains("Payload:"));
        assert!(prompt_output.contains("Type `run`"));
        assert!(!prompt_output.contains("hello from atctl"));

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "run".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert!(state.pending_payload.is_none());
        assert_eq!(
            transport.written_commands(),
            [
                "AT+CMGS=\"+819012345678\"\r".to_owned(),
                "hello from atctl\u{1a}".to_owned()
            ]
        );
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("+CMGS: 12"));
        assert!(output.contains("OK"));
    }

    #[test]
    fn unsupported_fixed_length_prompt_command_is_rejected_before_transport_access() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_response(b"\r\n> ".to_vec());
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+QISEND=0,4".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        assert!(state.pending_payload.is_none());
        assert!(state.pending_confirmation.is_none());
        assert!(transport.written_commands().is_empty());
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("requires a Sequence")
        );
    }

    #[test]
    fn prompt_payload_echo_is_masked_in_pty_output() {
        let mut state = BridgeCommandState::default();
        let mut transport = MockTransport::with_responses([
            b"\r\n> ".to_vec(),
            b"\r\nhello from atctl\r\n+CMGS: 12\r\n\r\nOK\r\n".to_vec(),
        ]);
        let mut output = Vec::new();

        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+CMGS=\"+819012345678\"".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "hello from atctl".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "run".to_owned(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("***** **** *****"));
        assert!(!output.contains("hello from atctl"));
    }

    #[cfg(unix)]
    #[test]
    fn bridge_raw_log_writes_exchange_without_unmasking_pty_output() {
        let mut state = BridgeCommandState::default();
        let mut transport =
            MockTransport::with_response(b"AT+CIMI\r\r\n295050912389644\r\n\r\nOK\r\n".to_vec());
        let mut output = Vec::new();
        let path = unique_temp_path("atctl-bridge-rawlog");
        let mut raw_log =
            RawLogSink::create(RawLogConfig::new(path.clone(), ExecutionSurface::Pty)).unwrap();

        let action = handle_bridge_line(
            &mut state,
            &mut transport,
            &mut output,
            "AT+CIMI".to_owned(),
            Duration::from_secs(30),
            Some(&mut raw_log),
        )
        .unwrap();

        assert_eq!(action, BridgeLoopAction::Continue);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("29505091******"));
        assert!(!output.contains("295050912389644"));
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"surface\":\"pty\""));
        assert!(raw.contains("\"operation\":{\"kind\":\"command\""));
        assert!(!raw.contains("\"source\""));
        assert!(raw.contains("295050912389644"));
        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_guard_refuses_existing_regular_file() {
        let link = unique_temp_path("atctl-bridge-existing-file");
        std::fs::write(&link, "not a symlink").unwrap();

        let error = SymlinkGuard::create(&link, &PathBuf::from("/dev/null"), true).unwrap_err();

        assert!(error.to_string().contains("refusing to overwrite"));
        let _ = std::fs::remove_file(link);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_guard_removes_only_matching_symlink() {
        let link = unique_temp_path("atctl-bridge-symlink");
        let target = PathBuf::from("/dev/null");
        {
            let guard = SymlinkGuard::create(&link, &target, false).unwrap();
            assert_eq!(std::fs::read_link(&link).unwrap(), target);
            drop(guard);
        }

        assert!(!Path::new(&link).exists());
    }

    #[cfg(unix)]
    fn unique_temp_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    struct BrokenPipeWriter;

    impl IoWrite for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> IoResult<usize> {
            Err(IoError::new(IoErrorKind::BrokenPipe, "client closed"))
        }

        fn flush(&mut self) -> IoResult<()> {
            Ok(())
        }
    }

    struct FlushBrokenPipeWriter;

    impl IoWrite for FlushBrokenPipeWriter {
        fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> IoResult<()> {
            Err(IoError::new(IoErrorKind::BrokenPipe, "client closed"))
        }
    }
}
