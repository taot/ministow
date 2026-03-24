fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to determine current directory: {err}");
            std::process::exit(1);
        }
    };

    let code = ministow::run_cli(&args, &cwd, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}
