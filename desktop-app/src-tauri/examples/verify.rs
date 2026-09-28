fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        eprintln!("Usage: verify REPOSITORY OUTPUT_DIRECTORY");
        std::process::exit(2)
    }
    let output = std::path::Path::new(&args[2]);
    std::fs::create_dir_all(output).unwrap();
    match mathlidelux_core::verification::verify_examples(std::path::Path::new(&args[1]), output) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1)
        }
    }
}
