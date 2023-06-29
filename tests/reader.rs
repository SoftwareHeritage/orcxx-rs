extern crate pretty_assertions;
extern crate orcxx;

use pretty_assertions::assert_eq;

use orcxx::*;

#[test]
fn nonexistent_file() {
    let stream_res = reader::InputStream::from_local_file("orc/examples/nonexistent.orc");
    assert!(matches!(stream_res, Err(utils::OrcError(_))));
}

#[test]
fn read_file() {
    let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
        .expect("Could not read");
    let reader = reader::Reader::new(input_stream);

    let expected_kind = kind::Kind::new(
        &r#"
        struct<
            boolean1:boolean,
            byte1:tinyint,
            short1:smallint,
            int1:int,
            long1:bigint,
            float1:float,
            double1:double,
            bytes1:binary,
            string1:string,
            middle:struct<
                list:array<
                    struct<
                        int1:int,
                        string1:string>>>,
            list:array<
                struct<
                    int1:int,
                    string1:string>>,
            map:map<
                string,
                struct<
                    int1:int,
                    string1:string>>>
        "#.split_whitespace().collect::<Vec<_>>().join("")).unwrap();

    assert_eq!(reader.kind(), expected_kind, "unexpected file structure");

    let mut row_reader = reader.row_reader(reader::RowReaderOptions::default());
    assert_eq!(row_reader.selected_kind(), expected_kind, "row_reader's selected type does not match the reader's type");

    let mut batch = row_reader.row_batch(1024);

    let mut total_elements = 0;
    let mut all_strings: Vec<String> = Vec::new();
    while row_reader.read_into(&mut batch) {
        total_elements += batch.num_elements();

        let struct_vector = batch
            .as_structs()
            .expect("could not cast ColumnVectorBatch to StructDataBuffer");
        let vectors = struct_vector.fields();

        for vector in vectors {
            match vector.as_strings() {
                Ok(string_vector) => {
                    for s in string_vector.iter() {
                        all_strings.push(std::str::from_utf8(s).unwrap_or("<not utf8>").to_owned())
                    }
                }
                Err(e) => println!("failed to cast to StringDataBuffer: {:?}", e),
            }
        }
    }

    assert_eq!(total_elements, 2);
    assert_eq!(
        all_strings,
        vec!["\0\u{1}\u{2}\u{3}\u{4}", "", "hi", "bye"]
            .iter()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>()
    );
}
