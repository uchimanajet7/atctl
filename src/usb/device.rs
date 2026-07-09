use serde::Serialize;

use crate::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsbDeviceFilter {
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub bus: Option<u8>,
    pub address: Option<u8>,
}

impl UsbDeviceFilter {
    pub fn matches(&self, device: &UsbDeviceInfo) -> bool {
        self.vendor_id
            .is_none_or(|vendor_id| device.vendor_id == vendor_id)
            && self
                .product_id
                .is_none_or(|product_id| device.product_id == product_id)
            && self.bus.is_none_or(|bus| device.bus == bus)
            && self.address.is_none_or(|address| device.address == address)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsbDeviceInfo {
    pub bus: u8,
    pub address: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub class_code: u8,
    pub sub_class_code: u8,
    pub protocol_code: u8,
    pub num_configurations: u8,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

pub fn read_device_info(device: &rusb::Device<rusb::GlobalContext>) -> Result<UsbDeviceInfo> {
    let descriptor = device.device_descriptor()?;
    let (manufacturer, product, serial_number) = read_string_descriptors(device, &descriptor);

    Ok(UsbDeviceInfo {
        bus: device.bus_number(),
        address: device.address(),
        vendor_id: descriptor.vendor_id(),
        product_id: descriptor.product_id(),
        class_code: descriptor.class_code(),
        sub_class_code: descriptor.sub_class_code(),
        protocol_code: descriptor.protocol_code(),
        num_configurations: descriptor.num_configurations(),
        manufacturer,
        product,
        serial_number,
    })
}

fn read_string_descriptors(
    device: &rusb::Device<rusb::GlobalContext>,
    descriptor: &rusb::DeviceDescriptor,
) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(handle) = device.open() else {
        return (None, None, None);
    };

    let manufacturer = handle.read_manufacturer_string_ascii(descriptor).ok();
    let product = handle.read_product_string_ascii(descriptor).ok();
    let serial_number = handle.read_serial_number_string_ascii(descriptor).ok();

    (manufacturer, product, serial_number)
}
