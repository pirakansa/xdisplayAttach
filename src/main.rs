use std::process::ExitCode;

fn main() -> ExitCode {
    match xdisplay_attach::run_cli() {
        Ok(result) => {
            for warning in result.warnings() {
                eprintln!("warning: {warning}");
            }
            let status = result.status();
            println!("{status}");
            ExitCode::from(status.code() as u8)
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code() as u8)
        }
    }
}
