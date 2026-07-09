use std::fmt;

use serde::Serialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EndpointDirection {
    In,
    Out,
}

impl fmt::Display for EndpointDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::In => formatter.write_str("in"),
            Self::Out => formatter.write_str("out"),
        }
    }
}

impl From<rusb::Direction> for EndpointDirection {
    fn from(direction: rusb::Direction) -> Self {
        match direction {
            rusb::Direction::In => Self::In,
            rusb::Direction::Out => Self::Out,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EndpointTransferType {
    Bulk,
    Interrupt,
    Isochronous,
    Control,
}

impl fmt::Display for EndpointTransferType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bulk => formatter.write_str("bulk"),
            Self::Interrupt => formatter.write_str("interrupt"),
            Self::Isochronous => formatter.write_str("isochronous"),
            Self::Control => formatter.write_str("control"),
        }
    }
}

impl From<rusb::TransferType> for EndpointTransferType {
    fn from(transfer_type: rusb::TransferType) -> Self {
        match transfer_type {
            rusb::TransferType::Bulk => Self::Bulk,
            rusb::TransferType::Interrupt => Self::Interrupt,
            rusb::TransferType::Isochronous => Self::Isochronous,
            rusb::TransferType::Control => Self::Control,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct EndpointDescriptor {
    pub address: u8,
    pub direction: EndpointDirection,
    pub transfer_type: EndpointTransferType,
    pub max_packet_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndpointPair {
    pub interface_number: u8,
    pub alternate_setting: Option<u8>,
    pub bulk_in: u8,
    pub bulk_out: u8,
    pub selection: EndpointSelection,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EndpointSelection {
    DescriptorShape,
    ManualOverride,
}

impl fmt::Display for EndpointSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptorShape => formatter.write_str("descriptor-shape"),
            Self::ManualOverride => formatter.write_str("manual-override"),
        }
    }
}

pub fn descriptor_shape_pairs(
    interface_number: u8,
    alternate_setting: u8,
    endpoints: &[EndpointDescriptor],
) -> Vec<EndpointPair> {
    let bulk_in_endpoints = endpoints
        .iter()
        .filter(|endpoint| {
            endpoint.direction == EndpointDirection::In
                && endpoint.transfer_type == EndpointTransferType::Bulk
        })
        .map(|endpoint| endpoint.address)
        .collect::<Vec<_>>();

    let bulk_out_endpoints = endpoints
        .iter()
        .filter(|endpoint| {
            endpoint.direction == EndpointDirection::Out
                && endpoint.transfer_type == EndpointTransferType::Bulk
        })
        .map(|endpoint| endpoint.address)
        .collect::<Vec<_>>();

    let mut pairs = Vec::new();
    for bulk_in in &bulk_in_endpoints {
        for bulk_out in &bulk_out_endpoints {
            pairs.push(EndpointPair {
                interface_number,
                alternate_setting: Some(alternate_setting),
                bulk_in: *bulk_in,
                bulk_out: *bulk_out,
                selection: EndpointSelection::DescriptorShape,
            });
        }
    }

    pairs
}

pub fn manual_override_pair(interface_number: u8, bulk_in: u8, bulk_out: u8) -> EndpointPair {
    EndpointPair {
        interface_number,
        alternate_setting: None,
        bulk_in,
        bulk_out,
        selection: EndpointSelection::ManualOverride,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_shape_pairs_bulk_in_and_out_endpoints() {
        let pairs = descriptor_shape_pairs(
            3,
            1,
            &[
                EndpointDescriptor {
                    address: 0x81,
                    direction: EndpointDirection::In,
                    transfer_type: EndpointTransferType::Bulk,
                    max_packet_size: 512,
                },
                EndpointDescriptor {
                    address: 0x02,
                    direction: EndpointDirection::Out,
                    transfer_type: EndpointTransferType::Bulk,
                    max_packet_size: 512,
                },
                EndpointDescriptor {
                    address: 0x83,
                    direction: EndpointDirection::In,
                    transfer_type: EndpointTransferType::Interrupt,
                    max_packet_size: 64,
                },
            ],
        );

        assert_eq!(
            pairs,
            vec![EndpointPair {
                interface_number: 3,
                alternate_setting: Some(1),
                bulk_in: 0x81,
                bulk_out: 0x02,
                selection: EndpointSelection::DescriptorShape,
            }]
        );
    }

    #[test]
    fn descriptor_shape_pairs_require_both_directions() {
        let pairs = descriptor_shape_pairs(
            0,
            0,
            &[EndpointDescriptor {
                address: 0x81,
                direction: EndpointDirection::In,
                transfer_type: EndpointTransferType::Bulk,
                max_packet_size: 512,
            }],
        );

        assert!(pairs.is_empty());
    }

    #[test]
    fn manual_override_pair_has_no_inferred_alternate_setting() {
        let pair = manual_override_pair(2, 0x85, 0x04);

        assert_eq!(pair.interface_number, 2);
        assert_eq!(pair.alternate_setting, None);
        assert_eq!(pair.selection, EndpointSelection::ManualOverride);
    }
}
