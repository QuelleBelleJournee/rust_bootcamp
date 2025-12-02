use clap::Parser;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::process;

/// Read and write binary files in hexadecimal
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Target file
    #[arg(short, long)]
    file: String,

    /// Read mode (display hex)
    #[arg(short, long, group = "action")]
    read: bool,

    /// Write mode (hex string to write)
    #[arg(short, long, group = "action")]
    write: Option<String>,

    /// Offset in bytes (decimal or 0x hex)
    #[arg(short, long, default_value = "0")]
    offset: String,

    /// Number of bytes to read
    #[arg(short, long)]
    size: Option<u64>,
}

fn main() {
    let args = Args::parse();

    // 1. Parsing de l'offset (décimal ou hexadécimal)
    let offset = match parse_offset(&args.offset) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error parsing offset: {}", e);
            process::exit(1);
        }
    };

    // 2. Mode Écriture (--write)
    if let Some(hex_str) = args.write {
        if let Err(e) = do_write(&args.file, offset, &hex_str) {
            eprintln!("Error writing file: {}", e);
            process::exit(1);
        }
    } 
    // 3. Mode Lecture (--read ou défaut si rien spécifié mais logique clap group)
    else if args.read {
        // Par défaut on lit 256 octets si --size n'est pas précisé, ou tout le fichier ?
        // L'image d'exemple montre --size 32 ou 16. Mettons une valeur par défaut raisonnable.
        let size = args.size.unwrap_or(256);
        if let Err(e) = do_read(&args.file, offset, size) {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    } else {
        // Si aucune action n'est fournie (bien que clap gère les groupes, c'est une sécurité)
        println!("Please specify --read or --write. Use --help for more info.");
    }
}

/// Parse un offset sous forme "100" (dec) ou "0x10" (hex)
fn parse_offset(input: &str) -> Result<u64, String> {
    let input = input.trim();
    if input.starts_with("0x") {
        u64::from_str_radix(&input[2..], 16)
            .map_err(|_| format!("Invalid hex offset: {}", input))
    } else {
        input.parse::<u64>()
            .map_err(|_| format!("Invalid decimal offset: {}", input))
    }
}

/// Convertit une chaine hex "48656c" en Vec<u8>
fn hex_string_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string length must be even".to_string());
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| format!("Invalid hex character at index {}", i))
        })
        .collect()
}

/// Logique de lecture (Hex dump)
fn do_read(path: &str, offset: u64, size: u64) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    
    // Seek vers l'offset
    file.seek(SeekFrom::Start(offset))?;

    // Lecture du buffer
    let mut handle = file.take(size);
    let mut buffer = Vec::new();
    handle.read_to_end(&mut buffer)?;

    // Affichage formaté (16 octets par ligne)
    for (i, chunk) in buffer.chunks(16).enumerate() {
        let current_offset = offset + (i as u64 * 16);
        
        // 1. Affichage de l'offset
        print!("{:08x}: ", current_offset);

        // 2. Affichage des octets en hex
        for byte in chunk {
            print!("{:02x} ", byte);
        }

        // Padding si la ligne est incomplète (pour aligner l'ASCII)
        for _ in 0..(16 - chunk.len()) {
            print!("   ");
        }

        print!("|");

        // 3. Affichage ASCII (printable ou '.')
        for byte in chunk {
            if *byte >= 0x20 && *byte <= 0x7E {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }

    Ok(())
}

/// Logique d'écriture
fn do_write(path: &str, offset: u64, hex_str: &str) -> Result<(), String> {
    let bytes = hex_string_to_bytes(hex_str)?;
    
    // Ouverture en mode write (et read pour ne pas tronquer si besoin, 
    // mais OpenOptions::write(true) sans truncate préserve le contenu existant)
    let mut file = OpenOptions::new()
        .write(true)
        .create(true) // Créer si n'existe pas
        .open(path)
        .map_err(|e| e.to_string())?;

    // Seek
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;

    // Écriture
    file.write_all(&bytes).map_err(|e| e.to_string())?;

    // Feedback utilisateur comme demandé dans l'image exemple
    println!("writing {} bytes at offset {:#010x}", bytes.len(), offset);
    print!("Hex: ");
    for b in &bytes { print!("{:02x} ", b); }
    println!();
    print!("ASCII: ");
    for b in &bytes {
        let c = if *b >= 0x20 && *b <= 0x7E { *b as char } else { '.' };
        print!("{}", c);
    }
    println!("\n✓ successfully written");

    Ok(())
}