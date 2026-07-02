use crate::config::read_config;
use crate::randr::X11Randr;
use crate::{
    AttachError, Command, CommandResult, ExitStatus, ModeRequest, OnRequest, OutputStatus, Result,
};
use std::env;
use std::path::PathBuf;

pub fn parse_args<I, S>(args: I) -> Result<Command>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(command) = args.next() else {
        return Err(AttachError::usage(usage()));
    };

    match command.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "status" => reject_extra(args, Command::Status),
        "on" => parse_on(args),
        "off" => parse_off(args),
        "auto" => parse_auto(args),
        _ => Err(AttachError::usage(format!(
            "unknown command '{command}'\n\n{}",
            usage()
        ))),
    }
}

pub fn usage() -> String {
    "Usage:
  xdisplay-attach status
  xdisplay-attach on --output NAME --preferred [--rotate DIR]
  xdisplay-attach on --output NAME --width N --height N [--rate HZ] [--rotate DIR]
  xdisplay-attach on --output NAME --rotate DIR
  xdisplay-attach off --output NAME
  xdisplay-attach auto --config FILE"
        .to_string()
}

pub fn run_cli() -> Result<CommandResult> {
    let command = parse_args(env::args().skip(1))?;
    match command {
        Command::Help => {
            println!("{}", usage());
            Ok(CommandResult::without_status_line(
                ExitStatus::AlreadySatisfied,
            ))
        }
        Command::Status => {
            let statuses = X11Randr::connect()?.status()?;
            for status in statuses {
                print!("{}", format_status(&status));
            }
            Ok(CommandResult::without_status_line(
                ExitStatus::AlreadySatisfied,
            ))
        }
        Command::On(request) => X11Randr::connect()?.turn_on(&request),
        Command::Off { output } => X11Randr::connect()?.turn_off(&output),
        Command::Auto { config } => {
            let config = read_config(&config)?;
            X11Randr::connect()?.auto(&config)
        }
    }
}

fn parse_on(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut output = None;
    let mut preferred = false;
    let mut width = None;
    let mut height = None;
    let mut rate = None;
    let mut rotation = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(next_value(&mut args, "--output")?),
            "--preferred" => preferred = true,
            "--width" => width = Some(parse_u16(next_value(&mut args, "--width")?, "--width")?),
            "--height" => height = Some(parse_u16(next_value(&mut args, "--height")?, "--height")?),
            "--rate" => rate = Some(parse_f64(next_value(&mut args, "--rate")?, "--rate")?),
            "--rotate" => {
                rotation = Some(parse_rotation(next_value(&mut args, "--rotate")?)?);
            }
            _ => return Err(AttachError::usage(format!("unknown on option '{arg}'"))),
        }
    }

    let output = output.ok_or_else(|| AttachError::usage("on requires --output"))?;
    let mode = match (preferred, width, height, rate, rotation) {
        (false, None, None, None, Some(_)) => ModeRequest::Current,
        (true, None, None, None, _) => ModeRequest::Preferred,
        (false, Some(width), Some(height), rate, _) => ModeRequest::Explicit {
            width,
            height,
            rate,
        },
        (true, Some(_), _, _, _) | (true, _, Some(_), _, _) | (true, _, _, Some(_), _) => Err(
            AttachError::usage("--preferred cannot be combined with --width, --height, or --rate"),
        )?,
        (false, _, _, Some(_), _) => {
            Err(AttachError::usage("--rate requires --width and --height"))?
        }
        _ => Err(AttachError::usage(
            "on requires --preferred, --width N --height N, or --rotate DIR",
        ))?,
    };

    Ok(Command::On(OnRequest {
        output,
        mode,
        x: 0,
        y: 0,
        rotation: rotation.unwrap_or(crate::RotationRequest::Normal),
    }))
}

fn parse_off(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut output = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(next_value(&mut args, "--output")?),
            _ => return Err(AttachError::usage(format!("unknown off option '{arg}'"))),
        }
    }
    Ok(Command::Off {
        output: output.ok_or_else(|| AttachError::usage("off requires --output"))?,
    })
}

fn parse_auto(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut config = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config = Some(PathBuf::from(next_value(&mut args, "--config")?)),
            _ => return Err(AttachError::usage(format!("unknown auto option '{arg}'"))),
        }
    }
    Ok(Command::Auto {
        config: config.ok_or_else(|| AttachError::usage("auto requires --config"))?,
    })
}

fn reject_extra(args: impl Iterator<Item = String>, command: Command) -> Result<Command> {
    let extras: Vec<String> = args.collect();
    if extras.is_empty() {
        Ok(command)
    } else {
        Err(AttachError::usage(format!(
            "unexpected argument '{}'",
            extras[0]
        )))
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| AttachError::usage(format!("{option} requires a value")))
}

fn parse_u16(value: String, option: &str) -> Result<u16> {
    value
        .parse()
        .map_err(|_| AttachError::usage(format!("{option} must be an integer")))
}

fn parse_f64(value: String, option: &str) -> Result<f64> {
    value
        .parse()
        .map_err(|_| AttachError::usage(format!("{option} must be a number")))
}

fn parse_rotation(value: String) -> Result<crate::RotationRequest> {
    match value.as_str() {
        "normal" => Ok(crate::RotationRequest::Normal),
        "left" => Ok(crate::RotationRequest::Left),
        "inverted" => Ok(crate::RotationRequest::Inverted),
        "right" => Ok(crate::RotationRequest::Right),
        _ => Err(AttachError::usage(
            "--rotate must be one of: normal, left, inverted, right",
        )),
    }
}

fn format_status(status: &OutputStatus) -> String {
    let connection = if status.connected {
        "connected"
    } else {
        "disconnected"
    };
    let activity = if status.active { "active" } else { "inactive" };
    let mut output = match (status.current_mode, status.x, status.y) {
        (Some(mode), Some(x), Some(y)) => {
            format!(
                "{} {connection} {activity} {}x{}+{x}+{y}",
                status.name, mode.width, mode.height
            )
        }
        _ => format!("{} {connection} {activity}", status.name),
    };

    for mode in &status.available_modes {
        output.push('\n');
        output.push_str("  ");
        output.push_str(&format_mode(mode, status.current_mode));
    }
    output.push('\n');
    output
}

fn format_mode(mode: &crate::ModeSummary, current_mode: Option<crate::ModeSummary>) -> String {
    let mut output = format!("{}x{}", mode.width, mode.height);
    if let Some(refresh_millihertz) = mode.refresh_millihertz {
        output.push(' ');
        output.push_str(&format_refresh(refresh_millihertz));
    }
    if current_mode.is_some_and(|current_mode| current_mode.mode_id == mode.mode_id) {
        output.push_str(" current");
    }
    if mode.preferred {
        output.push_str(" preferred");
    }
    output
}

fn format_refresh(refresh_millihertz: u32) -> String {
    format!(
        "{}.{:03}Hz",
        refresh_millihertz / 1000,
        refresh_millihertz % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, RotationRequest};

    #[test]
    fn parses_required_on_preferred_command() {
        let command = parse_args(["on", "--output", "HDMI-1", "--preferred"]).unwrap();
        assert_eq!(
            command,
            Command::On(OnRequest {
                output: "HDMI-1".to_string(),
                mode: ModeRequest::Preferred,
                x: 0,
                y: 0,
                rotation: RotationRequest::Normal
            })
        );
    }

    #[test]
    fn parses_on_rotate_command_using_current_mode() {
        let command = parse_args(["on", "--output", "HDMI-1", "--rotate", "left"]).unwrap();
        assert_eq!(
            command,
            Command::On(OnRequest {
                output: "HDMI-1".to_string(),
                mode: ModeRequest::Current,
                x: 0,
                y: 0,
                rotation: RotationRequest::Left
            })
        );
    }

    #[test]
    fn parses_on_preferred_with_rotation() {
        let command = parse_args([
            "on",
            "--output",
            "HDMI-1",
            "--preferred",
            "--rotate",
            "right",
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::On(OnRequest {
                output: "HDMI-1".to_string(),
                mode: ModeRequest::Preferred,
                x: 0,
                y: 0,
                rotation: RotationRequest::Right
            })
        );
    }

    #[test]
    fn rejects_preferred_with_explicit_mode() {
        let error = parse_args([
            "on",
            "--output",
            "HDMI-1",
            "--preferred",
            "--width",
            "1920",
            "--height",
            "1080",
        ])
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn parses_off_command() {
        assert_eq!(
            parse_args(["off", "--output", "HDMI-1"]).unwrap(),
            Command::Off {
                output: "HDMI-1".to_string()
            }
        );
    }

    #[test]
    fn parses_auto_command() {
        assert_eq!(
            parse_args(["auto", "--config", "displays.json"]).unwrap(),
            Command::Auto {
                config: PathBuf::from("displays.json")
            }
        );
    }

    #[test]
    fn rejects_status_extra_argument() {
        let error = parse_args(["status", "--output", "HDMI-1"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn rejects_rate_without_dimensions() {
        let error = parse_args(["on", "--output", "HDMI-1", "--rate", "60"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn rejects_missing_option_value() {
        let error = parse_args(["off", "--output"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn rejects_invalid_numeric_option() {
        let error = parse_args(["on", "--output", "HDMI-1", "--width", "wide"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn rejects_invalid_rotation() {
        let error = parse_args(["on", "--output", "HDMI-1", "--rotate", "sideways"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn formats_status_with_available_modes() {
        let status = OutputStatus {
            name: "HDMI-1".to_string(),
            connected: true,
            active: true,
            current_mode: Some(crate::ModeSummary {
                width: 1920,
                height: 1080,
                mode_id: 11,
                refresh_millihertz: Some(60_000),
                preferred: true,
            }),
            available_modes: vec![
                crate::ModeSummary {
                    width: 1920,
                    height: 1080,
                    mode_id: 11,
                    refresh_millihertz: Some(60_000),
                    preferred: true,
                },
                crate::ModeSummary {
                    width: 1280,
                    height: 720,
                    mode_id: 12,
                    refresh_millihertz: Some(59_940),
                    preferred: false,
                },
            ],
            x: Some(0),
            y: Some(0),
        };

        assert_eq!(
            format_status(&status),
            "HDMI-1 connected active 1920x1080+0+0\n  1920x1080 60.000Hz current preferred\n  1280x720 59.940Hz\n"
        );
    }

    #[test]
    fn formats_status_without_modes() {
        let status = OutputStatus {
            name: "DP-1".to_string(),
            connected: false,
            active: false,
            current_mode: None,
            available_modes: Vec::new(),
            x: None,
            y: None,
        };

        assert_eq!(format_status(&status), "DP-1 disconnected inactive\n");
    }
}
