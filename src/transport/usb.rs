use std::time::{Duration, Instant};

use crate::app::errors::AtctlError;
use crate::at::command::command_with_terminator;
use crate::at::parser::parse_response;
use crate::transport::traits::{AtTransport, ResponseMatcher};
use crate::usb::device::{UsbDeviceFilter, UsbDeviceInfo, read_device_info};
use crate::usb::endpoint::{EndpointDescriptor, EndpointPair, EndpointSelection};
use crate::{AtctlError as Error, Result};

const READ_BUFFER_SIZE: usize = 4096;

#[derive(Debug, Clone)]
pub struct UsbAtTransportConfig {
    pub filter: UsbDeviceFilter,
    pub manual_pair: Option<EndpointPair>,
    pub timeout: Duration,
    pub probe_timeout: Duration,
}

#[derive(Debug)]
pub struct UsbAtTransport {
    config: UsbAtTransportConfig,
    handle: Option<rusb::DeviceHandle<rusb::GlobalContext>>,
    selected_pair: Option<EndpointPair>,
}

#[derive(Debug, Clone)]
struct EndpointCandidate {
    configuration_value: u8,
    pair: EndpointPair,
}

impl UsbAtTransport {
    pub fn new(config: UsbAtTransportConfig) -> Self {
        Self {
            config,
            handle: None,
            selected_pair: None,
        }
    }
}

impl AtTransport for UsbAtTransport {
    fn open(&mut self) -> Result<()> {
        let (device, _device_info) = select_single_device(&self.config.filter)?;
        let handle = device.open()?;

        if rusb::supports_detach_kernel_driver() {
            let _ = handle.set_auto_detach_kernel_driver(true);
        }

        if let Some(pair) = self.config.manual_pair.clone() {
            claim_pair(&handle, &pair)?;
            self.selected_pair = Some(pair);
            self.handle = Some(handle);
            return Ok(());
        }

        let candidates = collect_endpoint_candidates(&device)?;
        for candidate in candidates {
            if probe_candidate(&handle, &candidate, self.config.probe_timeout).is_ok() {
                self.selected_pair = Some(candidate.pair);
                self.handle = Some(handle);
                return Ok(());
            }
        }

        Err(Error::EndpointDetectionFailed)
    }

    fn close(&mut self) -> Result<()> {
        if let (Some(handle), Some(pair)) = (&self.handle, &self.selected_pair) {
            let _ = handle.release_interface(pair.interface_number);
        }
        self.selected_pair = None;
        self.handle = None;
        Ok(())
    }

    fn write_command(&mut self, command: &str) -> Result<()> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| Error::Transport("USB transport is not open".to_owned()))?;
        let selected_pair = self
            .selected_pair
            .as_ref()
            .ok_or_else(|| Error::Transport("USB endpoint pair is not selected".to_owned()))?;
        write_bulk_command(
            handle,
            selected_pair.bulk_out,
            command.as_bytes(),
            self.config.timeout,
        )
    }

    fn read_response(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        self.read_until(timeout, ResponseMatcher::Terminal)
    }

    fn read_until(&mut self, timeout: Duration, matcher: ResponseMatcher) -> Result<Vec<u8>> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| Error::Transport("USB transport is not open".to_owned()))?;
        let selected_pair = self
            .selected_pair
            .as_ref()
            .ok_or_else(|| Error::Transport("USB endpoint pair is not selected".to_owned()))?;
        read_bulk_response_until(handle, selected_pair.bulk_in, timeout, matcher)
    }
}

impl Drop for UsbAtTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn select_single_device(
    filter: &UsbDeviceFilter,
) -> Result<(rusb::Device<rusb::GlobalContext>, UsbDeviceInfo)> {
    let devices = rusb::devices()?;
    let mut matches = Vec::new();

    for device in devices.iter() {
        let device_info = read_device_info(&device)?;
        if filter.matches(&device_info) {
            matches.push((device, device_info));
        }
    }

    match matches.len() {
        0 => Err(Error::DeviceNotFound),
        1 => Ok(matches.remove(0)),
        _ => Err(Error::MultipleDevices {
            devices: matches
                .iter()
                .map(|(_, device)| {
                    format!(
                        "{}:{} bus={} address={}",
                        hex_u16(device.vendor_id),
                        hex_u16(device.product_id),
                        device.bus,
                        device.address
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }),
    }
}

fn collect_endpoint_candidates(
    device: &rusb::Device<rusb::GlobalContext>,
) -> Result<Vec<EndpointCandidate>> {
    let descriptor = device.device_descriptor()?;
    let mut candidates = Vec::new();

    for index in 0..descriptor.num_configurations() {
        let config = device.config_descriptor(index)?;
        for interface in config.interfaces() {
            for interface_descriptor in interface.descriptors() {
                let endpoints = interface_descriptor
                    .endpoint_descriptors()
                    .map(|endpoint| EndpointDescriptor {
                        address: endpoint.address(),
                        direction: endpoint.direction().into(),
                        transfer_type: endpoint.transfer_type().into(),
                        max_packet_size: endpoint.max_packet_size(),
                    })
                    .collect::<Vec<_>>();

                for pair in crate::usb::endpoint::descriptor_shape_pairs(
                    interface_descriptor.interface_number(),
                    interface_descriptor.setting_number(),
                    &endpoints,
                ) {
                    candidates.push(EndpointCandidate {
                        configuration_value: config.number(),
                        pair,
                    });
                }
            }
        }
    }

    candidates.sort_by_key(candidate_sort_key);
    Ok(candidates)
}

fn candidate_sort_key(candidate: &EndpointCandidate) -> (u8, u8, u8, u8) {
    (
        candidate.configuration_value,
        candidate.pair.interface_number,
        candidate.pair.alternate_setting.unwrap_or_default(),
        candidate.pair.bulk_out,
    )
}

fn probe_candidate(
    handle: &rusb::DeviceHandle<rusb::GlobalContext>,
    candidate: &EndpointCandidate,
    timeout: Duration,
) -> Result<()> {
    claim_pair(handle, &candidate.pair)?;

    let result = write_bulk_command(
        handle,
        candidate.pair.bulk_out,
        command_with_terminator("AT").as_bytes(),
        timeout,
    )
    .and_then(|()| read_bulk_response(handle, candidate.pair.bulk_in, timeout))
    .map(|raw| parse_response(&raw))
    .and_then(|response| {
        if response.status.is_terminal() {
            Ok(())
        } else {
            Err(Error::Timeout)
        }
    });

    if result.is_err() {
        let _ = handle.release_interface(candidate.pair.interface_number);
    }

    result
}

fn claim_pair(handle: &rusb::DeviceHandle<rusb::GlobalContext>, pair: &EndpointPair) -> Result<()> {
    handle
        .claim_interface(pair.interface_number)
        .map_err(|source| AtctlError::InterfaceClaim {
            interface: pair.interface_number,
            source,
        })?;

    if pair.selection == EndpointSelection::DescriptorShape
        && let Some(alternate_setting) = pair.alternate_setting
        && alternate_setting != 0
        && let Err(source) = handle.set_alternate_setting(pair.interface_number, alternate_setting)
    {
        let _ = handle.release_interface(pair.interface_number);
        return Err(AtctlError::InterfaceClaim {
            interface: pair.interface_number,
            source,
        });
    }

    Ok(())
}

fn write_bulk_command(
    handle: &rusb::DeviceHandle<rusb::GlobalContext>,
    endpoint: u8,
    bytes: &[u8],
    timeout: Duration,
) -> Result<()> {
    let written = handle.write_bulk(endpoint, bytes, timeout)?;
    if written != bytes.len() {
        return Err(Error::Transport(format!(
            "partial USB bulk write: wrote {written} of {} bytes",
            bytes.len()
        )));
    }
    Ok(())
}

fn read_bulk_response(
    handle: &rusb::DeviceHandle<rusb::GlobalContext>,
    endpoint: u8,
    timeout: Duration,
) -> Result<Vec<u8>> {
    read_bulk_response_until(handle, endpoint, timeout, ResponseMatcher::Terminal)
}

fn read_bulk_response_until(
    handle: &rusb::DeviceHandle<rusb::GlobalContext>,
    endpoint: u8,
    timeout: Duration,
    matcher: ResponseMatcher,
) -> Result<Vec<u8>> {
    let started = Instant::now();
    let mut raw = Vec::new();

    loop {
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            return Err(Error::Timeout);
        }

        let remaining = timeout - elapsed;
        let mut buffer = [0_u8; READ_BUFFER_SIZE];

        match handle.read_bulk(endpoint, &mut buffer, remaining) {
            Ok(read) => {
                raw.extend_from_slice(&buffer[..read]);
                if matcher.is_match(&raw) {
                    return Ok(raw);
                }
            }
            Err(rusb::Error::Timeout) => return Err(Error::Timeout),
            Err(error) => return Err(error.into()),
        }
    }
}

fn hex_u16(value: u16) -> String {
    format!("0x{value:04x}")
}

#[cfg(test)]
mod tests {
    use crate::usb::endpoint::EndpointSelection;

    use super::*;

    #[test]
    fn sorts_candidates_by_configuration_interface_alt_and_out_endpoint() {
        let mut candidates = [
            EndpointCandidate {
                configuration_value: 1,
                pair: pair(3, 1, 0x85, 0x04),
            },
            EndpointCandidate {
                configuration_value: 1,
                pair: pair(2, 0, 0x82, 0x01),
            },
            EndpointCandidate {
                configuration_value: 2,
                pair: pair(0, 0, 0x81, 0x01),
            },
        ];

        candidates.sort_by_key(candidate_sort_key);

        assert_eq!(candidates[0].pair.interface_number, 2);
        assert_eq!(candidates[1].pair.interface_number, 3);
        assert_eq!(candidates[2].configuration_value, 2);
    }

    fn pair(
        interface_number: u8,
        alternate_setting: u8,
        bulk_in: u8,
        bulk_out: u8,
    ) -> EndpointPair {
        EndpointPair {
            interface_number,
            alternate_setting: Some(alternate_setting),
            bulk_in,
            bulk_out,
            selection: EndpointSelection::DescriptorShape,
        }
    }
}
