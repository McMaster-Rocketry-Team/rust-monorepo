use core::mem::size_of;
use crc::{Crc, CRC_16_USB};
use rkyv::{
    check_archived_root,
    ser::{serializers::BufferSerializer, Serializer},
    Archive, Deserialize,
};

use crate::packages::{
    ack::Ack,
    device::{DeviceInfo, GetDevice},
    event::{EventPackage, PollEvent},
    pyro::PyroCtrl,
    Package,
};

pub fn encode_package<'a, 'b, P: Package>(buffer: &'a mut [u8], package: P) -> &'a [u8] {
    let mut serializer = BufferSerializer::new([0u8; 128]);
    serializer.serialize_value(&package).unwrap();
    let serialized_package = serializer.into_inner();
    let serialized_package = &serialized_package[..size_of::<<P as Archive>::Archived>()];

    buffer[0] = serialized_package.len() as u8;
    buffer[3] = P::get_id();
    (&mut buffer[4..(4 + serialized_package.len())]).copy_from_slice(serialized_package);

    let crc = Crc::<u16>::new(&CRC_16_USB);
    let crc = crc.checksum(&buffer[3..(4 + serialized_package.len())]);
    (&mut buffer[1..3]).copy_from_slice(&crc.to_be_bytes());

    &buffer[..(4 + serialized_package.len())]
}

#[derive(defmt::Format, Debug)]
pub enum DecodedPackage {
    Ack(Ack),
    GetDevice(GetDevice),
    DeviceInfo(DeviceInfo),
    PyroCtrl(PyroCtrl),
    PollEvent(PollEvent),
    EventPackage(EventPackage),
}

#[derive(defmt::Format, Debug)]
pub enum DecodePackageError {
    LengthDoesNotMatch,
    CrcDoesNotMatch,
    StuctureDoesNotMatch,
    UnknownPackageId,
}

pub fn decode_package(buffer: &[u8]) -> Result<DecodedPackage, DecodePackageError> {
    let package_len = buffer[0] as usize;
    if buffer.len() != package_len + 4 {
        return Err(DecodePackageError::LengthDoesNotMatch);
    }
    let expected_crc = u16::from_be_bytes([buffer[1], buffer[2]]);

    let crc = Crc::<u16>::new(&CRC_16_USB);
    let actual_crc = crc.checksum(&buffer[3..]);
    if expected_crc != actual_crc {
        return Err(DecodePackageError::CrcDoesNotMatch);
    }

    let package_id = buffer[3];
    if package_id == Ack::get_id() {
        if let Ok(archived) = check_archived_root::<Ack>(&buffer[4..]) {
            return Ok(DecodedPackage::Ack(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    } else if package_id == GetDevice::get_id() {
        if let Ok(archived) = check_archived_root::<GetDevice>(&buffer[4..]) {
            return Ok(DecodedPackage::GetDevice(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    } else if package_id == DeviceInfo::get_id() {
        if let Ok(archived) = check_archived_root::<DeviceInfo>(&buffer[4..]) {
            return Ok(DecodedPackage::DeviceInfo(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    } else if package_id == PyroCtrl::get_id() {
        if let Ok(archived) = check_archived_root::<PyroCtrl>(&buffer[4..]) {
            return Ok(DecodedPackage::PyroCtrl(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    } else if package_id == PollEvent::get_id() {
        if let Ok(archived) = check_archived_root::<PollEvent>(&buffer[4..]) {
            return Ok(DecodedPackage::PollEvent(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    } else if package_id == EventPackage::get_id() {
        if let Ok(archived) = check_archived_root::<EventPackage>(&buffer[4..]) {
            return Ok(DecodedPackage::EventPackage(
                archived.deserialize(&mut rkyv::Infallible).unwrap(),
            ));
        }
        return Err(DecodePackageError::StuctureDoesNotMatch);
    }

    Err(DecodePackageError::UnknownPackageId)
}
