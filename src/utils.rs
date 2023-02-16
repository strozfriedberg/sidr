use chrono::prelude::*;

/// Converts a u64 filetime to a DateTime<Utc>
pub fn get_date_time_from_filetime(filetime: u64) -> DateTime<Utc> {
    const UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH: i128 = 11644473600;
    const UNIX_EPOCH_NANOS: i128 = UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH * 1_000_000_000;
    let filetime_nanos: i128 = filetime as i128 * 100;

    // Add nanoseconds to timestamp via Duration
    DateTime::<Utc>::from_utc(
        chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().and_hms_nano_opt(0, 0, 0, 0).unwrap()
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
pub fn find_guid(inp: &String, v: &str) -> String {
    let mut s = String::new();
    if let Some(i) = inp.find(v) {
        let start = i + v.len();
        if let Some(j) = inp[start..].find("}") {
            s = inp[start..start+j+1].into();
        }
    }
    s
}

pub fn from_utf16(val: &Vec<u8>) -> String {
    let s: Vec<u16> = val
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    String::from_utf16_lossy(s.as_slice())
}
