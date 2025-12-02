use clap::Parser;

#[derive(Parser)]
struct Args {
    // Le nom (par défaut "World")
    #[arg(default_value = "World")]
    name: String,

    // Le flag majuscule
    #[arg(long)]
    upper: bool,

    // Le nombre de répétitions (par défaut 1)
    #[arg(short, long, default_value_t = 1)]
    repeat: u8,
}

fn main() {
    let args = Args::parse();

    let name_to_display = if args.name.is_empty() {
        "World"
    } 
    else {
        &args.name
    };

    let mut message = format!("Hello, {}!", args.name);

    if args.upper {
        message = message.to_uppercase();
    }

    for _ in 0..args.repeat {
        println!("{}", message);
    }
}
