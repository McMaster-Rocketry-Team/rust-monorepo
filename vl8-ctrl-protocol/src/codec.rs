use core::mem::size_of;
use rkyv::{
    archived_root,
    ser::{serializers::BufferSerializer, Serializer},
    Archive, Deserialize,
};

use crate::packages::{
    ack::Ack,
    continuity::{ContinuityInfo, GetContinuity},
    device::{DeviceInfo, GetDevice},
    event::{EventPackage, PollEvent},
    hardware_arming::{GetHardwareArming, HardwareArmingInfo},
    pyro::PyroCtrl,
    Package,
};

pub fn encode_package<'a, 'b, P: Package>(buffer: &'a mut [u8], package: P) -> &'a [u8] {
    let mut serializer = BufferSerializer::new([0u8; 128]);
    serializer.serialize_value(&package).unwrap();
    let serialized_package = serializer.into_inner();
    let serialized_package = &serialized_package[..size_of::<<P as Archive>::Archived>()];

    buffer[0] = serialized_package.len() as u8;
    buffer[1] = P::get_id();
    (&mut buffer[2..(2 + serialized_package.len())]).copy_from_slice(serialized_package);

    &buffer[..(2 + serialized_package.len())]
}

#[derive(defmt::Format, Debug)]
pub enum DecodedPackage {
    Ack(Ack),
    GetDevice(GetDevice),
    DeviceInfo(DeviceInfo),
    PyroCtrl(PyroCtrl),
    PollEvent(PollEvent),
    EventPackage(EventPackage),
    GetContinuity(GetContinuity),
    ContinuityInfo(ContinuityInfo),
    GetHardwareArming(GetHardwareArming),
    HardwareArmingInfo(HardwareArmingInfo),
}

#[derive(defmt::Format, Debug)]
pub enum DecodePackageError {
    LengthDoesNotMatch,
    StuctureDoesNotMatch,
    UnknownPackageId,
}

pub fn decode_package(buffer: &[u8]) -> Result<DecodedPackage, DecodePackageError> {
    if buffer.len() == 0 {
        return Err(DecodePackageError::LengthDoesNotMatch);
    }
    let package_len = buffer[0] as usize;
    if buffer.len() != package_len + 2 {
        return Err(DecodePackageError::LengthDoesNotMatch);
    }

    let package_id = buffer[1];
    if package_id == Ack::get_id() {
        let archived = unsafe { archived_root::<Ack>(&buffer[2..]) };
        return Ok(DecodedPackage::Ack(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == GetDevice::get_id() {
        let archived = unsafe { archived_root::<GetDevice>(&buffer[2..]) };
        return Ok(DecodedPackage::GetDevice(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == DeviceInfo::get_id() {
        let archived = unsafe { archived_root::<DeviceInfo>(&buffer[2..]) };
        return Ok(DecodedPackage::DeviceInfo(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == PyroCtrl::get_id() {
        let archived = unsafe { archived_root::<PyroCtrl>(&buffer[2..]) };
        return Ok(DecodedPackage::PyroCtrl(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == PollEvent::get_id() {
        let archived = unsafe { archived_root::<PollEvent>(&buffer[2..]) };
        return Ok(DecodedPackage::PollEvent(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == EventPackage::get_id() {
        let archived = unsafe { archived_root::<EventPackage>(&buffer[2..]) };
        return Ok(DecodedPackage::EventPackage(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == GetContinuity::get_id() {
        let archived = unsafe { archived_root::<GetContinuity>(&buffer[2..]) };
        return Ok(DecodedPackage::GetContinuity(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == ContinuityInfo::get_id() {
        let archived = unsafe { archived_root::<ContinuityInfo>(&buffer[2..]) };
        return Ok(DecodedPackage::ContinuityInfo(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == GetHardwareArming::get_id() {
        let archived = unsafe { archived_root::<GetHardwareArming>(&buffer[2..]) };
        return Ok(DecodedPackage::GetHardwareArming(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    } else if package_id == HardwareArmingInfo::get_id() {
        let archived = unsafe { archived_root::<HardwareArmingInfo>(&buffer[2..]) };
        return Ok(DecodedPackage::HardwareArmingInfo(
            archived.deserialize(&mut rkyv::Infallible).unwrap(),
        ));
    }

    Err(DecodePackageError::UnknownPackageId)
}
