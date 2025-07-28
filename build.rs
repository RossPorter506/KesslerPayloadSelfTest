fn main() {
    let num_enabled_device_features = std::env::vars()
        .map(|(a, _)| a)
        .filter(|x| x.starts_with("CARGO_FEATURE_7"))
        .count();

    match num_enabled_device_features {
        0 => panic!("\x1b[31;1m No board feature enabled. Enable the feature that matches the serial number of your board. e.g. '--features 7B' \x1b[0m"),
        1 => (),
        _ => panic!("\x1b[31;1m Multiple board features enabled. Only use the board feature that matches the serial number of your board. \x1b[0m"),
    };
}