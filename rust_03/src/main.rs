use clap::{Parser, Subcommand};
use rand::Rng; // Nécessaire pour le trait .random()
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::process; // Pour exit(1)

// ==========================================
// 1. CONSTANTES & CONFIGURATION
// ==========================================

const P: u64 = 0xD87FA3E291B4C7F3; // Safe prime (64-bit)
const G: u64 = 2;                  // Generator

// Paramètres LCG
const LCG_A: u32 = 1103515245;
const LCG_C: u32 = 12345;

#[derive(Parser)]
#[command(name = "streamchat")]
#[command(about = "Stream cipher chat with Diffie-Hellman key generation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start server
    Server {
        #[arg(default_value_t = 8080)]
        port: u16,
    },
    /// Connect to server
    Client {
        host: String,
    },
}

// ==========================================
// 2. CRYPTO CORE
// ==========================================

fn mod_pow(base: u64, exp: u64, modulus: u64) -> u64 {
    let mut result: u128 = 1;
    let mut b: u128 = base as u128;
    let mut e = exp;
    let m = modulus as u128;

    while e > 0 {
        if (e % 2) == 1 {
            result = (result * b) % m;
        }
        b = (b * b) % m;
        e /= 2;
    }
    result as u64
}

struct LcgCipher {
    state: u32,
    count: usize,
}

impl LcgCipher {
    fn new(seed: u64) -> Self {
        println!("[STREAM] Generating keystream from secret...");
        println!("Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
        println!("Seed: secret = {:X}", seed);
        
        let state = seed as u32;
        
        print!("\nKeystream: ");
        let mut temp_state = state;
        for _ in 0..10 {
            temp_state = temp_state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
            let byte = (temp_state >> 24) as u8;
            print!("{:02X} ", byte);
        }
        println!("... \n");

        LcgCipher { state, count: 0 }
    }

    fn next_byte(&mut self) -> u8 {
        self.state = self.state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
        self.count += 1;
        (self.state >> 24) as u8 
    }

    fn process(&mut self, data: &[u8], mode: &str) -> Vec<u8> {
        let start_pos = self.count;
        let mut out = Vec::new();
        let mut key_bytes = Vec::new();

        for &b in data {
            let k = self.next_byte();
            key_bytes.push(k);
            out.push(b ^ k);
        }

        println!("[{}]", mode);
        if mode == "ENCRYPT" {
             print!("Plain: ");
             for b in data { print!("{:02x} ", b); }
             if let Ok(s) = std::str::from_utf8(data) { print!("({:?})", s); }
             println!();
        } else {
             print!("Cipher: ");
             for b in data { print!("{:02x} ", b); }
             println!();
        }

        print!("Key: ");
        for k in &key_bytes { print!("{:02x} ", k); }
        println!(" (keystream position: {})", start_pos);

        if mode == "ENCRYPT" {
            print!("Cipher: ");
            for b in &out { print!("{:02x} ", b); }
            println!();
        } else {
            print!("Plain: ");
            for b in &out { print!("{:02x} ", b); }
            if let Ok(s) = std::str::from_utf8(&out) { print!(" -> {:?}", s); }
            println!();
        }
        println!(); 

        out
    }
}

// ==========================================
// 3. LOGIQUE RESEAU
// ==========================================

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Server { port } => start_server(port),
        Commands::Client { host } => start_client(&host),
    }
}

fn handle_connection(mut stream: TcpStream) {
    let peer_addr = stream.peer_addr().unwrap();
    println!("[CLIENT] Connected from {}", peer_addr);

    // --- DH HANDSHAKE ---
    println!("\n[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("p = {:X} (64-bit prime - public)", P);
    println!("g = {} (generator - public)\n", G);

    println!("[DH] Generating our keypair...");
    // CORRECTION : Utilisation de rng().random() pour Rand 0.9+
    let private_key: u64 = rand::rng().random(); 
    println!("private_key = {:X} (random 64-bit)", private_key);

    let public_key = mod_pow(G, private_key, P);
    println!("public_key = g^private mod p");
    println!("= {}^{:X} mod p", G, private_key);
    println!("= {:X}\n", public_key);

    println!("[DH] Exchanging keys...");
    println!("[NETWORK] Sending public key (8 bytes)...");
    println!("-> Send our public: {:X}", public_key);
    if let Err(e) = stream.write_all(&public_key.to_be_bytes()) {
        eprintln!("Error sending key: {}", e);
        return;
    }

    let mut buffer = [0u8; 8];
    if let Err(e) = stream.read_exact(&mut buffer) {
        eprintln!("Error receiving key: {}", e);
        return;
    }
    let their_public_key = u64::from_be_bytes(buffer);
    println!("[NETWORK] Received public key (8 bytes) ✓");
    println!("<- Receive their public: {:X}\n", their_public_key);

    println!("[DH] Computing shared secret...");
    println!("Formula: secret = (their_public)^(our_private) mod p");
    let shared_secret = mod_pow(their_public_key, private_key, P);
    println!("secret = ({:X})^({:X}) mod p", their_public_key, private_key);
    println!("= {:X}\n", shared_secret);

    let mut cipher = LcgCipher::new(shared_secret);
    println!("✓ Secure channel established!\n");

    // --- CHAT LOOP ---
    let mut stream_reader = stream.try_clone().expect("Clone failed");
    let shared_secret_copy = shared_secret;
    
    // Thread de réception
    thread::spawn(move || {
        let mut decryptor = LcgCipher::new(shared_secret_copy);
        let mut buffer = [0u8; 1024];
        loop {
            match stream_reader.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    println!("\n[NETWORK] Received encrypted message ({} bytes)", n);
                    println!("[<-] Received {} bytes\n", n);
                    
                    let encrypted_data = &buffer[0..n];
                    decryptor.process(encrypted_data, "DECRYPT");
                    
                    print!("\n[CHAT] Type message:\n> ");
                    io::stdout().flush().unwrap();
                },
                Ok(_) => { println!("Peer disconnected."); process::exit(0); }
                Err(_) => { process::exit(0); }
            }
        }
    });

    // Boucle d'envoi
    loop {
        print!("[CHAT] Type message:\n> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        
        let trimmed = input.trim();
        if trimmed.is_empty() { continue; }

        let bytes = trimmed.as_bytes();
        let encrypted = cipher.process(bytes, "ENCRYPT");

        println!("[NETWORK] Sending encrypted message ({} bytes)...", encrypted.len());
        match stream.write_all(&encrypted) {
            Ok(_) => println!("[->] Sent {} bytes", encrypted.len()),
            Err(e) => { eprintln!("Send error: {}", e); break; }
        }
    }
}

fn start_server(port: u16) {
    // CORRECTION : Gestion propre de l'erreur de bind (Exit code 1)
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error: Could not bind to port {}. {}", port, e);
            process::exit(1);
        }
    };

    println!("[SERVER] Listening on 0.0.0.0:{}", port);
    println!("[SERVER] Waiting for client...\n");

    if let Ok((stream, _)) = listener.accept() {
        handle_connection(stream);
    }
}

fn start_client(host: &str) {
    println!("[CLIENT] Connecting to {}...", host);
    // CORRECTION : Gestion propre de l'erreur de connexion (Exit code 1)
    match TcpStream::connect(host) {
        Ok(stream) => {
            println!("[CLIENT] Connected!");
            handle_connection(stream);
        },
        Err(e) => {
            eprintln!("Error: Failed to connect to {}. {}", host, e);
            process::exit(1);
        },
    }
}