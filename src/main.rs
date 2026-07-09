use std::process::ExitCode;

fn main() -> ExitCode {
    atctl::init_tracing();

    match atctl::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
