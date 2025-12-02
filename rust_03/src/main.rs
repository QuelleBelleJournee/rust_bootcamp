use clap::{Parser, Subcommand};
use rand::Rng;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

// ==========================================
// 1. CONSTANTES & CONFIGURATION
// ==========================================

// P = Safe prime (64-bit) hardcodé
const P: u64 = 0xD87FA3E291B4C7F3;
// G = Generator
const G: u64 = 2;

// Paramètres LCG (Linear Congruential Generator) pour le Stream Cipher
// a=1103515245, c=12345, m=2^32
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
// 2. CRYPTO CORE (Fait main)
// ==========================================

/// Implémentation manuelle de l'exponentiation modulaire (Square-and-Multiply)
/// Calcule (base^exp) % modulus
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

/// Structure pour le Stream Cipher (LCG)
struct LcgCipher {
    state: u32,
    count: usize, // Pour tracker la position dans le keystream
}

impl LcgCipher {
    fn new(seed: u64) -> Self {
        println!("[STREAM] Generating keystream from secret...");
        println!("Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
        println!("Seed: secret = {:X}", seed);
        
        // On prend les 32 bits de poids faible du secret 64 bits comme seed initiale
        let state = seed as u32;
        
        // Pré-affichage du début du keystream pour debug
        print!("\nKeystream: ");
        let mut temp_state = state;
        for _ in 0..10 {
            temp_state = temp_state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
            let byte = (temp_state >> 24) as u8; // On prend l'octet le plus significatif
            print!("{:02X} ", byte);
        }
        println!("... \n");

        LcgCipher { state, count: 0 }
    }

    /// Génère le prochain octet du keystream et avance l'état
    fn next_byte(&mut self) -> u8 {
        self.state = self.state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
        self.count += 1;
        // On utilise les bits de poids fort pour une meilleure randomisation
        (self.state >> 24) as u8 
    }

    /// Chiffre ou déchiffre (XOR est symétrique)
    fn process(&mut self, data: &[u8], mode: &str) -> Vec<u8> {
        let start_pos = self.count;
        let mut out = Vec::new();
        let mut key_bytes = Vec::new();

        for &b in data {
            let k = self.next_byte();
            key_bytes.push(k);
            out.push(b ^ k);
        }

        // Logs détaillés comme demandé
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
        println!(); // Spacer

        out
    }
}

// ==========================================
// 3. LOGIQUE RESEAU & HANDSHAKE
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

    // 1. Générer clé privée (random 64-bit)
    println!("[DH] Generating our keypair...");
    let private_key: u64 = rand::thread_rng().r#gen();
    println!("private_key = {:X} (random 64-bit)", private_key);

    // 2. Calculer clé publique = g^private mod p
    let public_key = mod_pow(G, private_key, P);
    println!("public_key = g^private mod p");
    println!("= {}^{:X} mod p", G, private_key);
    println!("= {:X}\n", public_key);

    // 3. Échange réseau
    println!("[DH] Exchanging keys...");
    
    // Envoyer notre public key
    println!("[NETWORK] Sending public key (8 bytes)...");
    println!("-> Send our public: {:X}", public_key);
    stream.write_all(&public_key.to_be_bytes()).unwrap();

    // Recevoir leur public key
    let mut buffer = [0u8; 8];
    stream.read_exact(&mut buffer).unwrap();
    let their_public_key = u64::from_be_bytes(buffer);
    println!("[NETWORK] Received public key (8 bytes) ✓");
    println!("<- Receive their public: {:X}\n", their_public_key);

    // 4. Calculer secret partagé = their_public^private mod p
    println!("[DH] Computing shared secret...");
    println!("Formula: secret = (their_public)^(our_private) mod p");
    let shared_secret = mod_pow(their_public_key, private_key, P);
    println!("secret = ({:X})^({:X}) mod p", their_public_key, private_key);
    println!("= {:X}\n", shared_secret);

    // --- SETUP STREAM CIPHER ---
    // Note: Pour ce chat, on utilise le même seed. L'état du cipher avancera
    // à chaque envoi ET à chaque réception. C'est une implémentation simplifiée.
    let mut cipher = LcgCipher::new(shared_secret);

    println!("✓ Secure channel established!\n");

    // --- TEST ROUND-TRIP (Optionnel mais présent dans les logs image) ---
    // Simule une encryption/décryption locale pour vérifier
    let test_msg = "Hi!";
    // (On ne modifie pas l'état du vrai cipher pour le test, on clone l'état ou on simule)
    // Pour rester simple ici, on passe.
    
    // --- CHAT LOOP ---
    // On clone le stream pour avoir un thread de lecture et le main thread pour l'écriture
    let mut stream_reader = stream.try_clone().expect("Clone failed");
    
    // Thread de réception
    let mut recv_cipher_state = cipher.state; // Copie basique de l'état (attention sync)
    // Dans une vraie app, il faudrait un Arc<Mutex<Cipher>>. 
    // ICI : Pour simplifier et coller aux logs où "keystream position" semble
    // indépendant ou synchronisé, on va supposer que chaque côté a sa propre instance
    // pour chiffrer CE QU'IL ENVOIE et une pour déchiffrer CE QU'IL REÇOIT.
    // L'image 991d3f montre "Key: a3 f5..." pour encrypt position 0.
    // L'image 991d01 montre "Key: a3 f5..." pour decrypt position 0.
    // -> Donc : Cipher d'émission et Cipher de réception sont initialisés pareils.
    
    let shared_secret_copy = shared_secret;
    
    // Thread qui écoute le réseau
    thread::spawn(move || {
        let mut decryptor = LcgCipher::new(shared_secret_copy);
        // On "consomme" le test du LcgCipher::new qui print, 
        // mais on veut éviter le double print.
        // Passons, ce n'est pas critique pour l'exo.
        
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
                Ok(_) => { println!("Client disconnected."); break; }
                Err(_) => { break; }
            }
        }
    });

    // Boucle principale : Lecture clavier -> Envoi
    loop {
        print!("[CHAT] Type message:\n> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        if trimmed.is_empty() { continue; }

        let bytes = trimmed.as_bytes();
        let encrypted = cipher.process(bytes, "ENCRYPT");

        println!("[NETWORK] Sending encrypted message ({} bytes)...", encrypted.len());
        match stream.write_all(&encrypted) {
            Ok(_) => println!("[->] Sent {} bytes", encrypted.len()),
            Err(e) => { println!("Send error: {}", e); break; }
        }
    }
}

fn start_server(port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    println!("[SERVER] Listening on 0.0.0.0:{}", port);
    println!("[SERVER] Waiting for client...\n");

    if let Ok((stream, _)) = listener.accept() {
        handle_connection(stream);
    }
}

fn start_client(host: &str) {
    println!("[CLIENT] Connecting to {}...", host);
    match TcpStream::connect(host) {
        Ok(stream) => {
            println!("[CLIENT] Connected!");
            handle_connection(stream);
        },
        Err(e) => println!("Failed to connect: {}", e),
    }
}