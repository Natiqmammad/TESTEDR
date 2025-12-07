use exports_gen::{run, Args};

fn main() {
    if let Err(err) = run(Args::parse()) {
        eprintln!("error: {err:?}");
        std::process::exit(1);
    }
}
