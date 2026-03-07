fn main() {
    let code = match gx::run(std::env::args_os(), std::env::current_dir()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    };
    std::process::exit(code);
}
