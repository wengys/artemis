use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
    time::filetime_to_unixepoch,
};
use log::error;
use nom::bytes::complete::take;
use std::mem::size_of;

// There are three (3) Volume versions, however all of them have the same first 36 bytes
// Rest of bytes are unknown
// https://github.com/libyal/libscca/blob/main/documentation/Windows%20Prefetch%20File%20(PF)%20format.asciidoc#461-volume-information-entry
pub(crate) struct Volume {
    _volume_path_offset: u32,
    _volume_number_chars: u32,
    pub(crate) volume_path: String,
    pub(crate) volume_creation: i64,
    pub(crate) volume_serial: u32,
    _file_ref_offset: u32,
    _file_ref_data_size: u32,
    _directory_strings_offset: u32,
    pub(crate) number_directory_strings: u32,
    pub(crate) directories: Vec<String>,
}

impl Volume {
    /// Parse all Volume information entries
    pub(crate) fn parse_volume<'a>(
        data: &'a [u8],
        volume_offset: u32,
        number_volumes: &'a u32,
        version: u32,
    ) -> nom::IResult<&'a [u8], Vec<Volume>> {
        let mut volume_vec: Vec<Volume> = Vec::new();
        let mut count = 0;
        let (mut volume_data, _) = take(volume_offset)(data)?;
        let volume_start = volume_data;

        while &count < number_volumes {
            let (input, volume_path_offset) = nom_unsigned_four_bytes(volume_data, Endian::Le)?;
            let (input, volume_number_chars) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, volume_creation) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, volume_serial) = nom_unsigned_four_bytes(input, Endian::Le)?;

            let (input, file_ref_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, file_ref_data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, directory_strings_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, number_directory_strings) = nom_unsigned_four_bytes(input, Endian::Le)?;

            let (volume_path_start, _) = take(volume_path_offset)(volume_start)?;
            let utf16_adjust = 2;
            let (_, volume_path_data) =
                take(volume_number_chars * utf16_adjust)(volume_path_start)?;

            let (_, directories) = Volume::get_directories(
                volume_start,
                directory_strings_offset,
                number_directory_strings,
            )?;

            let volume = Volume {
                _volume_path_offset: volume_path_offset,
                _volume_number_chars: volume_number_chars,
                volume_path: extract_utf16_string(volume_path_data),
                volume_creation: filetime_to_unixepoch(&volume_creation),
                volume_serial,
                _file_ref_offset: file_ref_offset,
                _file_ref_data_size: file_ref_data_size,
                _directory_strings_offset: directory_strings_offset,
                number_directory_strings,
                directories,
            };
            volume_vec.push(volume);
            count += 1;

            let version30 = 30;
            let version26 = 26;
            let version23 = 23;
            volume_data = if version30 == version {
                let unknown_size: usize = 60;
                let (input, _) = take(unknown_size)(input)?;
                input
            } else if version26 == version || version23 == version {
                let unknown_size: usize = 68;
                let (input, _) = take(unknown_size)(input)?;
                input
            } else {
                error!("[prefetch] Unsupported prefetch volume info version: {version}");
                break;
            };
        }

        Ok((volume_data, volume_vec))
    }

    /// Get all the accessed directories
    fn get_directories(data: &[u8], offset: u32, entries: u32) -> nom::IResult<&[u8], Vec<String>> {
        let (mut directory_start, _) = take(offset)(data)?;

        let mut count = 0;
        let mut directories: Vec<String> = Vec::new();
        let utf16_adjust = 2;

        while count < entries {
            let (path_data, size) = nom_unsigned_two_bytes(directory_start, Endian::Le)?;
            let (remaining_data, path) = take(size * utf16_adjust)(path_data)?;

            // Nom end of string character (UTF16)
            let (remaining_data, _) = take(size_of::<u16>())(remaining_data)?;
            directory_start = remaining_data;
            directories.push(extract_utf16_string(path));
            count += 1;
        }
        Ok((directory_start, directories))
    }
}

#[cfg(test)]
mod tests {
    use super::Volume;

    #[test]
    fn test_parse_volume() {
        let test_data = vec![
            96, 0, 0, 0, 34, 0, 0, 0, 19, 157, 87, 144, 130, 130, 214, 1, 62, 147, 144, 66, 168, 0,
            0, 0, 184, 2, 0, 0, 96, 3, 0, 0, 15, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85,
            0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57,
            0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0,
            57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 0, 0, 0, 0, 3, 0, 0, 0, 85, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 244, 61, 9, 0, 0, 0, 0, 0, 248, 61, 9, 0, 0, 0, 0, 0, 252, 61, 9, 0, 0, 0,
            0, 0, 0, 62, 9, 0, 0, 0, 0, 0, 4, 62, 9, 0, 0, 0, 0, 0, 8, 62, 9, 0, 0, 0, 0, 0, 12,
            62, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 92, 0, 86, 0,
            79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0,
            56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0,
            50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 36, 0, 69, 0, 88, 0,
            84, 0, 69, 0, 78, 0, 68, 0, 0, 0, 46, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69,
            0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53,
            0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0,
            51, 0, 101, 0, 125, 0, 92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0, 65, 0, 77, 0, 68, 0,
            65, 0, 84, 0, 65, 0, 0, 0, 57, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123,
            0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0,
            57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0,
            101, 0, 125, 0, 92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0, 65, 0, 77, 0, 68, 0, 65, 0,
            84, 0, 65, 0, 92, 0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65, 0, 84, 0, 69, 0, 89,
            0, 0, 0, 63, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0,
            100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0,
            49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0,
            92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0, 65, 0, 77, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92,
            0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65, 0, 84, 0, 69, 0, 89, 0, 92, 0, 84, 0,
            79, 0, 79, 0, 76, 0, 83, 0, 0, 0, 40, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69,
            0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53,
            0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0,
            51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 0, 0, 44, 0, 92, 0,
            86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0,
            50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0,
            52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0,
            69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 0, 0, 52, 0, 92, 0, 86, 0, 79, 0, 76,
            0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50,
            0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0,
            48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0,
            92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 0,
            0, 58, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100,
            0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0,
            51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0,
            85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80,
            0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67, 0, 65, 0, 76, 0, 0, 0, 63, 0,
            92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0,
            56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0,
            45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0,
            83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68,
            0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67, 0, 65, 0, 76, 0, 92, 0, 84, 0, 69, 0,
            77, 0, 80, 0, 0, 0, 74, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48,
            0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0,
            100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0,
            125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 92, 0,
            65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67, 0, 65, 0, 76,
            0, 92, 0, 84, 0, 69, 0, 77, 0, 80, 0, 92, 0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0,
            65, 0, 84, 0, 69, 0, 89, 0, 0, 0, 86, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69,
            0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53,
            0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0,
            51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0,
            66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67,
            0, 65, 0, 76, 0, 92, 0, 84, 0, 69, 0, 77, 0, 80, 0, 92, 0, 67, 0, 72, 0, 79, 0, 67, 0,
            79, 0, 76, 0, 65, 0, 84, 0, 69, 0, 89, 0, 92, 0, 80, 0, 83, 0, 69, 0, 88, 0, 69, 0, 67,
            0, 46, 0, 50, 0, 46, 0, 52, 0, 48, 0, 0, 0, 42, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0,
            77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0,
            48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0,
            57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79, 0, 87, 0,
            83, 0, 0, 0, 51, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49,
            0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100,
            0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125,
            0, 92, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79, 0, 87, 0, 83, 0, 92, 0, 65, 0, 80, 0, 80, 0,
            80, 0, 65, 0, 84, 0, 67, 0, 72, 0, 0, 0, 51, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77,
            0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48,
            0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0,
            51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79, 0, 87, 0, 83, 0,
            92, 0, 83, 0, 89, 0, 83, 0, 84, 0, 69, 0, 77, 0, 51, 0, 50, 0, 0, 0, 51, 0, 92, 0, 86,
            0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50,
            0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0,
            50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0, 73, 0, 78, 0,
            68, 0, 79, 0, 87, 0, 83, 0, 92, 0, 83, 0, 89, 0, 83, 0, 87, 0, 79, 0, 87, 0, 54, 0, 52,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let volume_offset = 0;
        let volumes = 1;
        let (_, results) = Volume::parse_volume(&test_data, volume_offset, &volumes, 30).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].volume_path,
            "\\VOLUME{01d6828290579d13-4290933e}"
        );
        assert_eq!(results[0]._volume_path_offset, 96);
        assert_eq!(results[0]._volume_number_chars, 34);
        assert_eq!(results[0].volume_creation, 1599200033);
        assert_eq!(results[0].volume_serial, 0x4290933e);
        assert_eq!(results[0]._file_ref_offset, 168);
        assert_eq!(results[0]._file_ref_data_size, 696);
        assert_eq!(results[0]._directory_strings_offset, 864);
        assert_eq!(results[0].number_directory_strings, 15);

        assert_eq!(
            results[0].directories[0],
            "\\VOLUME{01d6828290579d13-4290933e}\\$EXTEND"
        );
        assert_eq!(
            results[0].directories[8],
            "\\VOLUME{01d6828290579d13-4290933e}\\USERS\\BOB\\APPDATA\\LOCAL\\TEMP"
        );
        assert_eq!(
            results[0].directories[14],
            "\\VOLUME{01d6828290579d13-4290933e}\\WINDOWS\\SYSWOW64"
        );
    }

    #[test]
    fn test_get_directories() {
        let test_data = vec![
            42, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0,
            54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0,
            51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0,
            36, 0, 69, 0, 88, 0, 84, 0, 69, 0, 78, 0, 68, 0, 0, 0, 46, 0, 92, 0, 86, 0, 79, 0, 76,
            0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50,
            0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0,
            48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0,
            65, 0, 77, 0, 68, 0, 65, 0, 84, 0, 65, 0, 0, 0, 57, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85,
            0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57,
            0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0,
            57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0, 65, 0,
            77, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65,
            0, 84, 0, 69, 0, 89, 0, 0, 0, 63, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0,
            123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0,
            55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0,
            51, 0, 101, 0, 125, 0, 92, 0, 80, 0, 82, 0, 79, 0, 71, 0, 82, 0, 65, 0, 77, 0, 68, 0,
            65, 0, 84, 0, 65, 0, 92, 0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65, 0, 84, 0, 69,
            0, 89, 0, 92, 0, 84, 0, 79, 0, 79, 0, 76, 0, 83, 0, 0, 0, 40, 0, 92, 0, 86, 0, 79, 0,
            76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0,
            50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0,
            57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0,
            83, 0, 0, 0, 44, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49,
            0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100,
            0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125,
            0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 0, 0, 52, 0,
            92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0,
            56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0,
            45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0,
            83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68,
            0, 65, 0, 84, 0, 65, 0, 0, 0, 58, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0,
            123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0,
            55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0,
            51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0,
            66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67,
            0, 65, 0, 76, 0, 0, 0, 63, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0,
            48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0,
            57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0,
            101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0,
            92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92, 0, 76, 0, 79, 0, 67, 0, 65,
            0, 76, 0, 92, 0, 84, 0, 69, 0, 77, 0, 80, 0, 0, 0, 74, 0, 92, 0, 86, 0, 79, 0, 76, 0,
            85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0,
            57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0,
            48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0, 82, 0, 83, 0,
            92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84, 0, 65, 0, 92,
            0, 76, 0, 79, 0, 67, 0, 65, 0, 76, 0, 92, 0, 84, 0, 69, 0, 77, 0, 80, 0, 92, 0, 67, 0,
            72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65, 0, 84, 0, 69, 0, 89, 0, 0, 0, 86, 0, 92, 0, 86,
            0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50,
            0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0,
            50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 85, 0, 83, 0, 69, 0,
            82, 0, 83, 0, 92, 0, 66, 0, 79, 0, 66, 0, 92, 0, 65, 0, 80, 0, 80, 0, 68, 0, 65, 0, 84,
            0, 65, 0, 92, 0, 76, 0, 79, 0, 67, 0, 65, 0, 76, 0, 92, 0, 84, 0, 69, 0, 77, 0, 80, 0,
            92, 0, 67, 0, 72, 0, 79, 0, 67, 0, 79, 0, 76, 0, 65, 0, 84, 0, 69, 0, 89, 0, 92, 0, 80,
            0, 83, 0, 69, 0, 88, 0, 69, 0, 67, 0, 46, 0, 50, 0, 46, 0, 52, 0, 48, 0, 0, 0, 42, 0,
            92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0,
            56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0,
            45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0,
            73, 0, 78, 0, 68, 0, 79, 0, 87, 0, 83, 0, 0, 0, 51, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85,
            0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57,
            0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0,
            57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79, 0, 87, 0,
            83, 0, 92, 0, 65, 0, 80, 0, 80, 0, 80, 0, 65, 0, 84, 0, 67, 0, 72, 0, 0, 0, 51, 0, 92,
            0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0, 49, 0, 100, 0, 54, 0, 56,
            0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0, 100, 0, 49, 0, 51, 0, 45, 0,
            52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0, 125, 0, 92, 0, 87, 0, 73, 0,
            78, 0, 68, 0, 79, 0, 87, 0, 83, 0, 92, 0, 83, 0, 89, 0, 83, 0, 84, 0, 69, 0, 77, 0, 51,
            0, 50, 0, 0, 0, 51, 0, 92, 0, 86, 0, 79, 0, 76, 0, 85, 0, 77, 0, 69, 0, 123, 0, 48, 0,
            49, 0, 100, 0, 54, 0, 56, 0, 50, 0, 56, 0, 50, 0, 57, 0, 48, 0, 53, 0, 55, 0, 57, 0,
            100, 0, 49, 0, 51, 0, 45, 0, 52, 0, 50, 0, 57, 0, 48, 0, 57, 0, 51, 0, 51, 0, 101, 0,
            125, 0, 92, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79, 0, 87, 0, 83, 0, 92, 0, 83, 0, 89, 0,
            83, 0, 87, 0, 79, 0, 87, 0, 54, 0, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let volume_offset = 0;
        let volumes = 15;
        let (_, results) = Volume::get_directories(&test_data, volume_offset, volumes).unwrap();
        assert_eq!(results.len(), 15);

        assert_eq!(
            results[2],
            "\\VOLUME{01d6828290579d13-4290933e}\\PROGRAMDATA\\CHOCOLATEY"
        );
        assert_eq!(
            results[6],
            "\\VOLUME{01d6828290579d13-4290933e}\\USERS\\BOB\\APPDATA"
        );
        assert_eq!(results[11], "\\VOLUME{01d6828290579d13-4290933e}\\WINDOWS");
        assert_eq!(
            results[13],
            "\\VOLUME{01d6828290579d13-4290933e}\\WINDOWS\\SYSTEM32"
        );
    }
}
