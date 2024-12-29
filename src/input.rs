use input::{Libinput, LibinputInterface};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::OwnedFd};
use std::path::Path;

// stolen from linux/input.h
mod codes {
    pub const W: u32 = 17;
    pub const S: u32 = 31;
    pub const UP: u32 = 103;
    pub const DOWN: u32 = 108;
    pub const SPACE: u32 = 57;
}

pub enum Key {
    W,
    S,
    Up,
    Down,
    Space,
}

impl TryFrom<u32> for Key {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            codes::W => Ok(Self::W),
            codes::S => Ok(Self::S),
            codes::SPACE => Ok(Self::Space),
            codes::UP => Ok(Self::Up),
            codes::DOWN => Ok(Self::Down),
            _ => Err(()),
        }
    }
}

pub struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
    }
}

impl Interface {
    pub fn new() -> Libinput {
        let mut input = Libinput::new_with_udev(Self);
        input.udev_assign_seat("seat0").unwrap();
        input
    }
}
