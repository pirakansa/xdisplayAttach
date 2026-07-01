use crate::config::read_config;
use crate::randr::X11Randr;
use crate::{AttachError, Command, ExitStatus, ModeRequest, OnRequest, OutputStatus, Result};
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
  xdisplay-attach on --output NAME --preferred
  xdisplay-attach on --output NAME --width N --height N [--rate HZ]
  xdisplay-attach off --output NAME
  xdisplay-attach auto --config FILE"
        .to_string()
}

pub fn run_cli() -> Result<ExitStatus> {
    let command = parse_args(env::args().skip(1))?;
    match command {
        Command::Help => {
            println!("{}", usage());
            Ok(ExitStatus::AlreadySatisfied)
        }
        Command::Status => {
            let statuses = X11Randr::connect()?.status()?;
            for status in statuses {
                print_status(&status);
            }
            Ok(ExitStatus::AlreadySatisfied)
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
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(next_value(&mut args, "--output")?),
            "--preferred" => preferred = true,
            "--width" => width = Some(parse_u16(next_value(&mut args, "--width")?, "--width")?),
            "--height" => height = Some(parse_u16(next_value(&mut args, "--height")?, "--height")?),
            "--rate" => rate = Some(parse_f64(next_value(&mut args, "--rate")?, "--rate")?),
            _ => return Err(AttachError::usage(format!("unknown on option '{arg}'"))),
        }
    }

    let output = output.ok_or_else(|| AttachError::usage("on requires --output"))?;
    let mode = match (preferred, width, height, rate) {
        (true, None, None, None) => ModeRequest::Preferred,
        (false, Some(width), Some(height), rate) => ModeRequest::Explicit {
            width,
            height,
            rate,
        },
        (true, Some(_), _, _) | (true, _, Some(_), _) | (true, _, _, Some(_)) => Err(
            AttachError::usage("--preferred cannot be combined with --width, --height, or --rate"),
        )?,
        (false, _, _, Some(_)) => Err(AttachError::usage("--rate requires --width and --height"))?,
        _ => Err(AttachError::usage(
            "on requires either --preferred or --width N --height N",
        ))?,
    };

    Ok(Command::On(OnRequest {
        output,
        mode,
        x: 0,
        y: 0,
        rotation: crate::RotationRequest::Normal,
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

fn print_status(status: &OutputStatus) {
    let connection = if status.connected {
        "connected"
    } else {
        "disconnected"
    };
    let activity = if status.active { "active" } else { "inactive" };
    match (status.current_mode, status.x, status.y) {
        (Some(mode), Some(x), Some(y)) => {
            println!(
                "{} {connection} {activity} {}x{}+{x}+{y}",
                status.name, mode.width, mode.height
            );
        }
        _ => println!("{} {connection} {activity}", status.name),
    }
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
}
