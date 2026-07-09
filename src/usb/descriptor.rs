use serde::Serialize;

use crate::usb::device::UsbDeviceInfo;
use crate::usb::endpoint::{EndpointDescriptor, EndpointPair, descriptor_shape_pairs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsbInspection {
    pub device: UsbDeviceInfo,
    pub configurations: Vec<ConfigDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigDescriptor {
    pub configuration_value: u8,
    pub self_powered: bool,
    pub remote_wakeup: bool,
    pub max_power_ma: u16,
    pub interfaces: Vec<InterfaceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceDescriptor {
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub class_code: u8,
    pub sub_class_code: u8,
    pub protocol_code: u8,
    pub endpoints: Vec<EndpointDescriptor>,
    pub descriptor_shape_pairs: Vec<EndpointPair>,
}

impl InterfaceDescriptor {
    pub fn new(
        interface_number: u8,
        alternate_setting: u8,
        class_code: u8,
        sub_class_code: u8,
        protocol_code: u8,
        endpoints: Vec<EndpointDescriptor>,
    ) -> Self {
        let descriptor_shape_pairs =
            descriptor_shape_pairs(interface_number, alternate_setting, &endpoints);

        Self {
            interface_number,
            alternate_setting,
            class_code,
            sub_class_code,
            protocol_code,
            endpoints,
            descriptor_shape_pairs,
        }
    }
}
