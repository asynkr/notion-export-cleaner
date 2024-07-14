const CHARS_KEPT_BY_URI_ENCODING: &str = "-_.!~*'();/?:@&=+$,#";

/// URI encoding is URL encoding but with some characters kept
pub fn encode_uri(text: &str) -> String {
    let url_encoded = urlencoding::encode(text).into_owned();

    let mut uri_encoded = url_encoded;
    for character in CHARS_KEPT_BY_URI_ENCODING.chars() {
        let char_string = character.to_string();
        let char_encoded = urlencoding::encode(&char_string);
        uri_encoded = uri_encoded.replace(&char_encoded.into_owned(), &char_string);
    }
    uri_encoded
}
