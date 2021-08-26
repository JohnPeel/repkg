use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

mod parser {
    use nom::{
        bytes::complete::{tag, take_until},
        combinator::{map_res, verify},
        multi::{many1, many_m_n},
        number::complete::{le_u16, le_u32, le_u8},
        sequence::{preceded, terminated, tuple},
        IResult,
    };

    #[derive(Debug)]
    pub struct Header {
        // magic: [u8; 4],
        pub version: u32,
        pub file_data_offset: usize,
        pub number_of_files: usize,
        pub directory_records_offset: usize,
        pub number_of_directory_records: usize,
        pub name_directory_offset: usize,
        pub file_type_directory_offset: usize,
        // padding: [u8; 480]
    }

    #[derive(Debug)]
    pub struct FileRecord {
        // null0: u8
        pub file_type_offset: usize,
        // null1: u8
        pub file_name_offset: usize,
        pub file_data_offset: usize,
        pub file_data_size: usize,
    }

    #[derive(Debug, Clone)]
    pub struct DirectoryRecord {
        pub characters: Vec<char>,
        // null0: u8
        pub link_1: usize,
        pub link_2: usize,
        pub record_id: u16,
        pub start_file_index: usize,
        pub end_file_index: usize,
    }

    pub fn parse_header(input: &[u8]) -> IResult<&[u8], Header> {
        let (
            input,
            (
                version,
                file_data_offset,
                number_of_files,
                directory_records_offset,
                number_of_directory_records,
                name_directory_offset,
                file_type_directory_offset,
            ),
        ) = preceded(
            tag("ZPKG"),
            terminated(
                tuple((le_u32, le_u32, le_u32, le_u32, le_u32, le_u32, le_u32)),
                many_m_n(480, 480, verify(le_u8, |b| *b == 0)),
            ),
        )(input)?;

        Ok((
            input,
            Header {
                version,
                file_data_offset: file_data_offset as usize,
                number_of_files: number_of_files as usize,
                directory_records_offset: directory_records_offset as usize,
                number_of_directory_records: number_of_directory_records as usize,
                name_directory_offset: name_directory_offset as usize,
                file_type_directory_offset: file_type_directory_offset as usize,
            },
        ))
    }

    pub fn parse_file_record(input: &[u8]) -> IResult<&[u8], FileRecord> {
        let (input, (_, file_type_offset, _, file_name_offset, file_data_offset, file_data_size)) =
            tuple((
                verify(le_u8, |b| *b == 0),
                le_u16,
                verify(le_u8, |b| *b == 0),
                le_u32,
                le_u32,
                le_u32,
            ))(input)?;

        Ok((
            input,
            FileRecord {
                file_type_offset: file_type_offset as usize,
                file_name_offset: file_name_offset as usize,
                file_data_offset: file_data_offset as usize,
                file_data_size: file_data_size as usize,
            },
        ))
    }

    pub fn parse_file_records(input: &[u8]) -> IResult<&[u8], Vec<FileRecord>> {
        many1(parse_file_record)(input)
    }

    pub fn parse_directory_record(input: &[u8]) -> IResult<&[u8], DirectoryRecord> {
        let (input, (character, _, link_1, link_2, record_id, start_file_index, end_file_index)) =
            tuple((
                le_u8,
                verify(le_u8, |b| *b == 0),
                le_u16,
                le_u16,
                le_u16,
                le_u16,
                le_u16,
            ))(input)?;

        Ok((
            input,
            DirectoryRecord {
                characters: vec![character as char],
                link_1: link_1 as usize,
                link_2: link_2 as usize,
                record_id,
                start_file_index: start_file_index as usize,
                end_file_index: end_file_index as usize,
            },
        ))
    }

    pub fn parse_directory_records(input: &[u8]) -> IResult<&[u8], Vec<DirectoryRecord>> {
        many1(parse_directory_record)(input)
    }

    pub fn parse_zstr(input: &[u8]) -> IResult<&[u8], &str> {
        map_res(
            terminated(take_until("\0"), tag("\0")),
            core::str::from_utf8,
        )(input)
    }
}

#[derive(Debug)]
pub struct ZpkgFile {
    pub path: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct Zpkg {
    pub version: u32,
    pub files: Vec<ZpkgFile>,
}

impl Zpkg {
    pub fn from_slice(input: &[u8]) -> Result<Zpkg, BoxError> {
        let (input, header) = parser::parse_header(input)
            .map_err::<BoxError, _>(|_err| "Unable to parse pkg header.".into())?;

        let (file_records, input) = input.split_at(header.directory_records_offset - 512);
        let (directory_records, input) =
            input.split_at(header.name_directory_offset - header.directory_records_offset);
        let (name_directory, input) =
            input.split_at(header.file_type_directory_offset - header.name_directory_offset);
        let (file_type_directory, file_data) =
            input.split_at(header.file_data_offset - header.file_type_directory_offset);

        let (remaining, file_records) = parser::parse_file_records(file_records)
            .map_err::<BoxError, _>(|_err| "Unable to parse file records.".into())?;
        assert_eq!(0, remaining.len());
        assert_eq!(header.number_of_files, file_records.len());

        let (remaining, mut directory_records) = parser::parse_directory_records(directory_records)
            .map_err::<BoxError, _>(|_err| "Unable to parse directory records.".into())?;
        assert_eq!(0, remaining.len());
        assert_eq!(header.number_of_directory_records, directory_records.len());

        let mut directory_map: HashMap<usize, String> = HashMap::with_capacity(file_records.len());
        let mut directory_name = vec!['\x02', '/'];
        for index in 0..directory_records.len() {
            let record = directory_records.get(index).unwrap().clone();

            for link in [record.link_1, record.link_2] {
                if link != 0 {
                    let other = directory_records.get_mut(link).unwrap();
                    if !directory_name.is_empty() {
                        other
                            .characters
                            .splice(0..0, directory_name.iter().cloned());
                    } else {
                        other.characters.splice(
                            0..0,
                            (&record.characters[..record.characters.len() - 1])
                                .iter()
                                .cloned(),
                        );
                    }
                }
            }

            directory_name.extend(record.characters);

            if record.end_file_index != 0 {
                for index in record.start_file_index..record.end_file_index {
                    assert!(!directory_map.contains_key(&index));
                    directory_map.insert(index, directory_name.iter().skip(1).collect());
                }

                if let Some(parser::DirectoryRecord { characters, .. }) =
                    directory_records.get(index + 1)
                {
                    if characters.contains(&'\x02') {
                        directory_name.clear();
                    }
                }
            }
        }

        let mut files = Vec::with_capacity(file_records.len());
        for (index, file_record) in file_records.into_iter().enumerate() {
            let data_offset = file_record.file_data_offset - header.file_data_offset;
            let file_name = parser::parse_zstr(&name_directory[file_record.file_name_offset..])
                .map_err::<BoxError, _>(|_err| "Unable to parse file name.".into())?
                .1;
            let file_ext = parser::parse_zstr(&file_type_directory[file_record.file_type_offset..])
                .map_err::<BoxError, _>(|_err| "Unable to parse file extension.".into())?
                .1;
            let path = format!(
                "{}/{}.{}",
                directory_map.get(&index).unwrap_or(&"".to_string()),
                file_name,
                file_ext
            );
            let data = (&file_data[data_offset..data_offset + file_record.file_data_size]).to_vec();

            files.push(ZpkgFile { path, data });
        }

        Ok(Zpkg {
            version: header.version,
            files,
        })
    }
}
