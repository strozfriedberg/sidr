use chrono::prelude::*;

use std::convert::TryInto;

/// Converts a u64 filetime to a DateTime<Utc>
pub fn get_date_time_from_filetime(filetime: u64) -> DateTime<Utc> {
    const UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH: i128 = 11644473600;
    const UNIX_EPOCH_NANOS: i128 = UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH * 1_000_000_000;
    let filetime_nanos: i128 = filetime as i128 * 100;

    // Add nanoseconds to timestamp via Duration
    DateTime::<Utc>::from_utc(
        chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_nano_opt(0, 0, 0, 0)
            .unwrap()
            + chrono::Duration::nanoseconds((filetime_nanos - UNIX_EPOCH_NANOS) as i64),
        Utc,
    )
}

/// Converts a DateTime<Utc> to ISO-8601/RFC-3339 format `%Y-%m-%dT%H:%M:%S%.7f` (manually, since Rust doesn't support `%.7f`)
pub fn format_date_time(date_time: DateTime<Utc>) -> String {
    let fractional_seconds = date_time.format("%9f").to_string();
    const EXPECTED_FRACTIONAL_SECONDS_LEN: usize = 9;
    if EXPECTED_FRACTIONAL_SECONDS_LEN == fractional_seconds.len() {
        let byte_slice = fractional_seconds.as_bytes(); // we know that the string is only ASCII, so this is safe
                                                        // Make sure that our last two digits are 0, as we expect
                                                        // Note that we aren't just using chrono::SecondsFormat::AutoSi because we want 7 digits to correspond to the original filetime's 100ns precision
        if byte_slice[EXPECTED_FRACTIONAL_SECONDS_LEN - 1] == b'0'
            && byte_slice[EXPECTED_FRACTIONAL_SECONDS_LEN - 2] == b'0'
        {
            return format!(
                "{}.{}Z",
                date_time.format("%Y-%m-%dT%H:%M:%S"),
                &fractional_seconds[..7]
            );
        }
    }
    // We should nenver hit this when coming from a FILETIME; we don't have that much precision
    date_time.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

// extract GUID string from string like:
// file:///C:/Users/testuser/Desktop/Test-Word.docx?VolumeId={AC048C6D-1E3C-4B21-B20D-75745DD788B3}&ObjectId={5E5EFB20-A904-11ED-A0EA-DC215CBBECEC}&KnownFolderId=ThisPCDesktopFolder&KnownFolderLength=25
pub fn find_guid(inp: &str, v: &str) -> String {
    let mut s = String::new();
    if let Some(i) = inp.find(v) {
        let start = i + v.len();
        if let Some(j) = inp[start..].find('}') {
            s = inp[start..start + j + 1].into();
        }
    }
    s
}

pub fn from_utf16(val: &[u8]) -> String {
    let s: Vec<u16> = val
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    String::from_utf16_lossy(s.as_slice())
}

bitflags! {
    struct file_attributes_flag: u32 {
        const FILE_ATTRIBUTE_READONLY              = 0x00000001;
        const FILE_ATTRIBUTE_HIDDEN                = 0x00000002;
        const FILE_ATTRIBUTE_SYSTEM                = 0x00000004;
        const FILE_ATTRIBUTE_DIRECTORY             = 0x00000010;
        const FILE_ATTRIBUTE_ARCHIVE               = 0x00000020;
        const FILE_ATTRIBUTE_DEVICE                = 0x00000040;
        const FILE_ATTRIBUTE_NORMAL                = 0x00000080;
        const FILE_ATTRIBUTE_TEMPORARY             = 0x00000100;
        const FILE_ATTRIBUTE_SPARSE_FILE           = 0x00000200;
        const FILE_ATTRIBUTE_REPARSE_POINT         = 0x00000400;
        const FILE_ATTRIBUTE_COMPRESSED            = 0x00000800;
        const FILE_ATTRIBUTE_OFFLINE               = 0x00001000;
        const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED   = 0x00002000;
        const FILE_ATTRIBUTE_ENCRYPTED             = 0x00004000;
        const FILE_ATTRIBUTE_INTEGRITY_STREAM      = 0x00008000;
        const FILE_ATTRIBUTE_VIRTUAL               = 0x00010000;
        const FILE_ATTRIBUTE_NO_SCRUB_DATA         = 0x00020000;
        const FILE_ATTRIBUTE_PINNED                = 0x00080000;
        const FILE_ATTRIBUTE_UNPINNED              = 0x00100000;
        const FILE_ATTRIBUTE_RECALL_ON_OPEN        = 0x00040000;
        const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS = 0x00400000;
    }
}

pub fn file_attributes_to_string(bytes: &Vec<u8>) -> String {
    let at = if bytes.len() == 1 {
        u8::from_le_bytes(bytes[..].try_into().unwrap()) as u32
    } else if bytes.len() == 2 {
        u16::from_le_bytes(bytes[..].try_into().unwrap()) as u32
    } else if bytes.len() == 4 {
        u32::from_le_bytes(bytes[..].try_into().unwrap())
    } else {
        return format!("{:?}", bytes);
    };
    let f = unsafe { file_attributes_flag::from_bits_unchecked(at) };
    format!("{:?}", f)
}

// in: 15F-System_DateModified
// out: System_DateModified
pub fn column_string_part(s: &str) -> &str {
    match s.find('-') {
        Some(i) => &s[i + 1..],
        None => s,
    }
}
