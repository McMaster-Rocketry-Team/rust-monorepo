use super::*;

pub(super) fn find_most_common_u16_out_of_4(buffer: &[u8]) -> Option<u16> {
    find_most_common(
        u16::from_be_bytes((&buffer[0..2]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[2..4]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[4..6]).try_into().unwrap()),
        u16::from_be_bytes((&buffer[6..8]).try_into().unwrap()),
    )
}

fn find_most_common(a: u16, b: u16, c: u16, d: u16) -> Option<u16> {
    if a == b {
        return Some(a);
    }
    if a == c {
        return Some(a);
    }
    if a == d {
        return Some(a);
    }
    if b == c {
        return Some(b);
    }
    if b == d {
        return Some(b);
    }
    if c == d {
        return Some(c);
    }

    None
}

pub trait CopyFromU16x4 {
    fn copy_from_u16x4(&mut self, value: u16);
}

impl CopyFromU16x4 for [u8] {
    fn copy_from_u16x4(&mut self, value: u16) {
        let be_bytes = &value.to_be_bytes();
        (&mut self[0..2]).copy_from_slice(be_bytes);
        (&mut self[2..4]).copy_from_slice(be_bytes);
        (&mut self[4..6]).copy_from_slice(be_bytes);
        (&mut self[6..8]).copy_from_slice(be_bytes);
    }
}

impl<F, C> VLFS<F, C>
where
    F: Flash,
    C: Crc,
{
    pub(super) fn find_file_entry<'a>(
        &self,
        allocation_table: &'a AllocationTable,
        file_id: FileID,
    ) -> Option<&'a FileEntry> {
        for file_entry in &allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Some(file_entry);
            }
        }
        None
    }

    pub(super) fn find_file_entry_mut<'a>(
        &self,
        allocation_table: &'a mut AllocationTable,
        file_id: FileID,
    ) -> Option<&'a mut FileEntry> {
        for file_entry in &mut allocation_table.file_entries {
            if file_entry.file_id == file_id {
                return Some(file_entry);
            }
        }
        None
    }
}
