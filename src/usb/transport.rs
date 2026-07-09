use crate::Result;
use crate::usb::descriptor::{ConfigDescriptor, InterfaceDescriptor, UsbInspection};
use crate::usb::device::{UsbDeviceFilter, UsbDeviceInfo, read_device_info};
use crate::usb::endpoint::EndpointDescriptor;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UsbDeviceListMode {
    AtTargets,
    AllUsb,
}

pub fn list_devices(
    filter: &UsbDeviceFilter,
    mode: UsbDeviceListMode,
) -> Result<Vec<UsbDeviceInfo>> {
    let devices = rusb::devices()?;
    let mut matching_devices = Vec::new();

    for device in devices.iter() {
        let device_info = read_device_info(&device)?;
        if !filter.matches(&device_info) {
            continue;
        }
        if mode == UsbDeviceListMode::AtTargets && !is_at_operation_target(&device, &device_info)? {
            continue;
        }

        matching_devices.push(device_info);
    }

    Ok(matching_devices)
}

fn is_at_operation_target(
    device: &rusb::Device<rusb::GlobalContext>,
    device_info: &UsbDeviceInfo,
) -> Result<bool> {
    if !is_at_candidate_device_class(device_info.class_code) {
        return Ok(false);
    }

    for index in 0..device_info.num_configurations {
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

                if !crate::usb::endpoint::descriptor_shape_pairs(
                    interface_descriptor.interface_number(),
                    interface_descriptor.setting_number(),
                    &endpoints,
                )
                .is_empty()
                {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn is_at_candidate_device_class(class_code: u8) -> bool {
    matches!(class_code, 0x02 | 0xef | 0xff)
}

pub fn inspect_devices(filter: &UsbDeviceFilter) -> Result<Vec<UsbInspection>> {
    let devices = rusb::devices()?;
    let mut inspections = Vec::new();

    for device in devices.iter() {
        let device_info = read_device_info(&device)?;
        if !filter.matches(&device_info) {
            continue;
        }

        let mut configurations = Vec::new();
        for index in 0..device_info.num_configurations {
            let config = device.config_descriptor(index)?;
            let interfaces = config
                .interfaces()
                .flat_map(|interface| interface.descriptors())
                .map(|interface| {
                    let endpoints = interface
                        .endpoint_descriptors()
                        .map(|endpoint| EndpointDescriptor {
                            address: endpoint.address(),
                            direction: endpoint.direction().into(),
                            transfer_type: endpoint.transfer_type().into(),
                            max_packet_size: endpoint.max_packet_size(),
                        })
                        .collect::<Vec<_>>();

                    InterfaceDescriptor::new(
                        interface.interface_number(),
                        interface.setting_number(),
                        interface.class_code(),
                        interface.sub_class_code(),
                        interface.protocol_code(),
                        endpoints,
                    )
                })
                .collect();

            configurations.push(ConfigDescriptor {
                configuration_value: config.number(),
                self_powered: config.self_powered(),
                remote_wakeup: config.remote_wakeup(),
                max_power_ma: config.max_power(),
                interfaces,
            });
        }

        inspections.push(UsbInspection {
            device: device_info,
            configurations,
        });
    }

    Ok(inspections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_candidate_device_class_is_conservative() {
        assert!(is_at_candidate_device_class(0x02));
        assert!(is_at_candidate_device_class(0xef));
        assert!(is_at_candidate_device_class(0xff));
        assert!(!is_at_candidate_device_class(0x00));
        assert!(!is_at_candidate_device_class(0x09));
        assert!(!is_at_candidate_device_class(0x11));
    }
}
