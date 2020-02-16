// hda_analyzer_rs is a tool to analyze HDA codecs, widgets, and connections
// Copyright (C) 2020 Jeremy Cline
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along
// with this program; if not, write to the Free Software Foundation, Inc.,
// 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::str;
use std::os::raw::c_int;

use nix::{ioctl_read, ioctl_read_bad, ioctl_readwrite};

const VERBS_PARAMETERS: u32 = 0x0f00;
const PARAMS_NODE_COUNT: u32 = 0x04;

fn from_nix_error(err: ::nix::Error) -> io::Error {
    io::Error::from_raw_os_error(
        err.as_errno()
            .unwrap_or_else(|| nix::errno::Errno::UnknownErrno) as i32,
    )
}

fn from_nix_result<T>(res: ::nix::Result<T>) -> io::Result<T> {
    match res {
        Ok(r) => Ok(r),
        Err(err) => Err(from_nix_error(err)),
    }
}

#[repr(C)]
pub struct c_hda_info {
    pub device_number: u32,
    pub card_number: i32,
    _id: [u8; 64],        // string len 64
    hwdep_name: [u8; 80], // len 80
    hwdep_interface: i32,
    reserved: [u8; 64], // len 64
}

pub struct DeviceInfo {
    pub device_number: u32,
    pub card_number: i32,
    pub id: String,
    pub name: String,
    pub interface: i32,
}

// #[derive(Debug, Default)]
#[repr(C)]
pub struct c_card_info {
    pub card: i32,         // Card number
    pad: i32,              // Pad, unused for now.
    id: [u8; 16],          // ID of the card, user selectable
    driver: [u8; 16],      // Driver name
    name: [u8; 32],        // short name of the sound card
    longname: [u8; 80],    // name + info text about sound card
    reserved_: [u8; 16],   // Was ID of mixer, now unused
    mixername: [u8; 80],   // visual mixer identification
    components: [u8; 128], // card components/ fine identification, delimited with one space (AC97 etc...)
}

pub struct CardInfo {
    pub card: i32,         // Card number
    pub id: String,          // ID of the card, user selectable
    pub driver: String,      // Driver name
    pub name: String,        // short name of the sound card
    pub longname: String,    // name + info text about sound card
    pub mixername: String,   // visual mixer identification
    pub components: String, // card components/ fine identification, delimited with one space (AC97 etc...)
}

impl Default for c_card_info {
    fn default() -> Self {
        Self {
            card: -1,
            pad: 0,
            id: [0; 16],
            driver: [0; 16],
            name: [0; 32],
            longname: [0; 80],
            reserved_: [0; 16],
            mixername: [0; 80],
            components: [0; 128],
        }
    }
}

impl c_card_info {
    fn to_cardinfo(&self) -> CardInfo {
        CardInfo {
            card: self.card,
            id: str::from_utf8(&self.id).unwrap().to_owned(),
            driver: str::from_utf8(&self.driver).unwrap().to_owned(),
            name: str::from_utf8(&self.name).unwrap().to_owned(),
            longname: str::from_utf8(&self.longname).unwrap().to_owned(),
            mixername: str::from_utf8(&self.mixername).unwrap().to_owned(),
            components: str::from_utf8(&self.components).unwrap().to_owned(),
        }
    }
}

impl Default for c_hda_info {
    fn default() -> Self {
        Self {
            device_number: 0,
            card_number: -1,
            _id: [0; 64],
            hwdep_name: [0; 80],
            hwdep_interface: 0,
            reserved: [0; 64],
        }
    }
}

impl c_hda_info {
    fn to_deviceinfo(&self) -> DeviceInfo {
        DeviceInfo {
            device_number: self.device_number,
            card_number: self.card_number,
            id: str::from_utf8(&self._id).unwrap().to_owned(),
            name: str::from_utf8(&self.hwdep_name).unwrap().to_owned(),
            interface: self.hwdep_interface,
        }
    }
}

#[repr(C)]
pub struct hda_verb {
    verb: u32, // NID << 24 | verb << 8 | param
    response: u32,
}

impl hda_verb {
    pub fn new(node_id: u32, verb: u32, param: u32) -> hda_verb {
        hda_verb {
            verb: (node_id << 24) | (verb << 8) | param,
            response: 0,
        }
    }

    pub fn sub_node_count(codec: &File, node_id: u32) -> (u32, u32) {
        let response = write_verb(codec, node_id, VERBS_PARAMETERS, PARAMS_NODE_COUNT).unwrap();
        (response & 0x7fff, (response >> 16) & 0x7fff)
    }
}

pub fn get_hda_info(card: &File) -> io::Result<DeviceInfo> {
    let mut info: c_hda_info = Default::default();
    let fd = card.as_raw_fd();
    from_nix_result(unsafe { hda_info(fd, &mut info) })?;
    Ok(info.to_deviceinfo())
}

pub fn get_hda_card_info(card: &File) -> io::Result<CardInfo> {
    let mut card_info: c_card_info = Default::default();
    let fd = card.as_raw_fd();
    from_nix_result(unsafe { hda_card_info(fd, &mut card_info) })?;
    Ok(card_info.to_cardinfo())
}

pub fn write_verb(codec: &File, node_id: u32, verb: u32, param: u32) -> io::Result<u32> {
    let mut v = hda_verb::new(node_id, verb, param);
    let fd = codec.as_raw_fd();
    from_nix_result(unsafe { hda_verb_write(fd, &mut v) })?;
    Ok(v.response)
}

const HDA_IOCTL_INFO: u32 = 0x80dc_4801;
const HDA_IOCTL_CARD_INFO_MAGIC: u8 = b'U';
const HDA_IOCTL_CARD_INFO_TYPE_MODE: u8 = 0x01;
const HDA_IOCTL_PVERSION_MAGIC: u8 = b'H';
const HDA_IOCTL_PVERSION_TYPE_MODE: u8 = 0x10;
const HDA_IOCTL_GET_WCAP_MAGIC: u8 = b'H';
const HDA_IOCTL_GET_WCAP_TYPE_MODE: u8 = 0x12;
ioctl_read_bad!(hda_info, HDA_IOCTL_INFO, c_hda_info);
ioctl_read!(
    hda_card_info,
    HDA_IOCTL_CARD_INFO_MAGIC,
    HDA_IOCTL_CARD_INFO_TYPE_MODE,
    c_card_info
);
ioctl_read!(
    hda_pversion,
    HDA_IOCTL_PVERSION_MAGIC,
    HDA_IOCTL_PVERSION_TYPE_MODE,
    c_int
);
ioctl_readwrite!(hda_verb_write, b'H', 0x11, hda_verb);
ioctl_readwrite!(
    hda_get_wcap,
    HDA_IOCTL_GET_WCAP_MAGIC,
    HDA_IOCTL_GET_WCAP_TYPE_MODE,
    hda_verb
);

// pub struct Codec {
//     card: i32,
//     device: i32,
//     vendor_id: i32,
//     subsystem_id: i32,
//     revision_id: i32,
//     audio_function_group: i32,  // Probably actually a vector of structs for these
//     modem_function_group: i32,
// }
//
// impl Codec {
//     pub fn new(card: i32, device: i32) -> Codec {
//         // Open /dev/snd/hwC{card}D{device}
//         Codec {
//             card,
//             device,
//             vendor_id: 0xffff,
//             subsystem_id: 0xffff,
//             revision_id: 0xffff,
//             audio_function_group: 0,
//             modem_function_group: 0,
//         }
//     }
// }
