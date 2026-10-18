//! Checked decoding for untrusted Win32 buffer lengths. No aligned byte casts.
use std::mem::{offset_of, size_of};
use windows::Win32::UI::Input::{RAWINPUT, RAWINPUTHEADER, RAWMOUSE, RIM_TYPEMOUSE};

/// DWORD/pointer-aligned output storage, initialized so unwritten bytes are safe to inspect.
pub(crate) struct InputBuffer(Vec<u64>);
impl InputBuffer {
    pub(crate) fn new(bytes: u32) -> Option<Self> {
        let words = (bytes as usize).checked_add(7)? / 8;
        let mut storage = Vec::new();
        storage.try_reserve_exact(words).ok()?;
        storage.resize(words, 0);
        Some(Self(storage))
    }
    pub(crate) fn as_mut_ptr(&mut self) -> *mut core::ffi::c_void {
        self.0.as_mut_ptr().cast()
    }
    pub(crate) fn bytes(&self, count: u32) -> Option<&[u8]> {
        if count as usize > self.0.len().checked_mul(8)? {
            return None;
        }
        // u64 has no padding or invalid bit patterns, and every word is initialized.
        Some(unsafe { std::slice::from_raw_parts(self.0.as_ptr().cast(), count as usize) })
    }
}
fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

pub(crate) fn relative_mouse(bytes: &[u8]) -> Option<(i32, i32)> {
    if bytes.len() < size_of::<RAWINPUTHEADER>() {
        return None;
    }
    if u32_at(bytes, offset_of!(RAWINPUTHEADER, dwType))? != RIM_TYPEMOUSE.0 {
        return None;
    }
    let declared = u32_at(bytes, offset_of!(RAWINPUTHEADER, dwSize))? as usize;
    let data = offset_of!(RAWINPUT, data);
    if declared > bytes.len() || declared < data.checked_add(size_of::<RAWMOUSE>())? {
        return None;
    }
    let flags = u16::from_le_bytes(bytes.get(data..data + 2)?.try_into().ok()?);
    if flags & 1 != 0 {
        return None;
    } // MOUSE_MOVE_ABSOLUTE
    let x = u32_at(bytes, data + offset_of!(RAWMOUSE, lLastX))? as i32;
    let y = u32_at(bytes, data + offset_of!(RAWMOUSE, lLastY))? as i32;
    Some((x, y))
}

pub(crate) fn property_string(bytes: &[u8], returned: u32) -> Option<String> {
    let bytes = bytes.get(..returned as usize)?;
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    if units.last() != Some(&0) {
        return None;
    }
    let end = units.iter().position(|u| *u == 0)?;
    if end == 0 || end != units.len() - 1 {
        return None;
    }
    String::from_utf16(&units[..end]).ok()
}

pub(crate) fn terminated(units: &[u16]) -> Option<&[u16]> {
    let end = units.iter().position(|u| *u == 0)?;
    (end > 0).then_some(&units[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mouse() -> Vec<u8> {
        let mut b = vec![0; offset_of!(RAWINPUT, data) + size_of::<RAWMOUSE>()];
        let len = b.len() as u32;
        b[4..8].copy_from_slice(&len.to_le_bytes());
        let data = offset_of!(RAWINPUT, data);
        b[data + offset_of!(RAWMOUSE, lLastX)..][..4].copy_from_slice(&(-7i32).to_le_bytes());
        b[data + offset_of!(RAWMOUSE, lLastY)..][..4].copy_from_slice(&12i32.to_le_bytes());
        b
    }
    #[test]
    fn mouse_decode_checks_type_lengths_and_absolute_flag() {
        let b = mouse();
        assert_eq!(relative_mouse(&b), Some((-7, 12)));
        let mut odd = vec![0];
        odd.extend_from_slice(&b);
        assert_eq!(relative_mouse(&odd[1..]), Some((-7, 12)));
        for len in 0..b.len() {
            assert!(relative_mouse(&b[..len]).is_none());
        }
        let mut bad = b.clone();
        bad[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(relative_mouse(&bad).is_none());
        let mut bad = b.clone();
        bad[4..8].copy_from_slice(&8u32.to_le_bytes());
        assert!(relative_mouse(&bad).is_none());
        let mut bad = b.clone();
        bad[0..4].copy_from_slice(&1u32.to_le_bytes());
        assert!(relative_mouse(&bad).is_none());
        let mut bad = b;
        bad[offset_of!(RAWINPUT, data)] = 1;
        assert!(relative_mouse(&bad).is_none());
    }
    #[test]
    fn property_decode_rejects_changed_sizes_odd_bytes_and_missing_terminators() {
        let b = [b'a', 0, 0, 0];
        assert_eq!(property_string(&b, 4).as_deref(), Some("a"));
        let mut odd = vec![9];
        odd.extend_from_slice(&b);
        assert_eq!(property_string(&odd[1..], 4).as_deref(), Some("a"));
        for len in [0, 1, 2, 3, 5, u32::MAX] {
            assert!(property_string(&b, len).is_none());
        }
        assert!(property_string(&[0, 0], 2).is_none());
        assert!(property_string(&[0, 0xd8, 0, 0], 4).is_none());
        assert!(property_string(&[b'a', 0, 0, 0, b'b', 0, 0, 0], 8).is_none());
    }
    #[test]
    fn raw_storage_is_aligned_and_bounded() {
        let b = InputBuffer::new(9).unwrap();
        assert_eq!(b.0.as_ptr() as usize % std::mem::align_of::<RAWINPUT>(), 0);
        assert_eq!(b.bytes(9).unwrap(), &[0; 9]);
        assert!(b.bytes(17).is_none());
        assert!(terminated(&[1, 2]).is_none());
        assert_eq!(terminated(&[1, 0, 2]), Some(&[1u16][..]));
    }
}
