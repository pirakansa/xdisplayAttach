use xdisplay_attach::{parse_args, Command, ModeRequest};

#[test]
fn parses_status_command() {
    assert_eq!(parse_args(["status"]).unwrap(), Command::Status);
}

#[test]
fn parses_explicit_on_command() {
    let command = parse_args([
        "on", "--output", "HDMI-1", "--width", "1920", "--height", "1080", "--rate", "60",
    ])
    .unwrap();

    let Command::On(request) = command else {
        panic!("expected on command");
    };
    assert_eq!(request.output, "HDMI-1");
    assert_eq!(
        request.mode,
        ModeRequest::Explicit {
            width: 1920,
            height: 1080,
            rate: Some(60.0)
        }
    );
}
