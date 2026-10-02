fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("Usage: verify REPOSITORY OUTPUT_DIRECTORY [TYPST_EXECUTABLE]");
        std::process::exit(2)
    }
    let output = std::path::Path::new(&args[2]);
    std::fs::create_dir_all(output).unwrap();
    let repository = std::path::Path::new(&args[1]);
    let result = if args.len() == 4 {
        mathlidelux_core::verification::verify_typst_version(
            repository,
            output,
            std::path::Path::new(&args[3]),
        )
    } else {
        mathlidelux_core::verification::verify_examples(repository, output)
    };
    match result {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1)
        }
    }
}
