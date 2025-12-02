use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};

/// Count word frequency in text
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Text to analyze (or use stdin)
    text: Option<String>,

    /// Show top N words
    #[arg(long, default_value_t = 10)]
    top: usize,

    /// Ignore words shorter than N
    #[arg(long, default_value_t = 1)]
    min_length: usize,

    /// Case insensitive counting
    #[arg(long)]
    ignore_case: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // 1. Récupération du contenu (Argument direct OU Stdin)
    let content = match args.text {
        Some(text) => text,
        None => {
            // Si pas d'argument texte, on lit stdin
            let mut buffer = String::new();
            // On vérifie si stdin est interactif pour éviter de bloquer si vide
            // (Note: pour une pipeline simple 'cat file | cargo run', read_to_string suffit)
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    // 2. Comptage des mots
    let mut counts: HashMap<String, usize> = HashMap::new();

    // On découpe par tout ce qui n'est pas alphanumérique (pour virer la ponctuation)
    let tokens = content
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty());

    for token in tokens {
        // Filtre de longueur
        if token.len() < args.min_length {
            continue;
        }

        // Gestion de la casse
        let word = if args.ignore_case {
            token.to_lowercase()
        } else {
            token.to_string()
        };

        *counts.entry(word).or_insert(0) += 1;
    }

    // 3. Tri des résultats
    let mut sorted_counts: Vec<(&String, &usize)> = counts.iter().collect();

    // Tri principal : Fréquence (décroissant)
    // Tri secondaire : Alphabétique (pour avoir un ordre stable en cas d'égalité)
    sorted_counts.sort_by(|a, b| {
        b.1.cmp(a.1).then_with(|| a.0.cmp(b.0))
    });

    // 4. Affichage
    // On détermine le titre en fonction du contexte (comme sur les screenshots)
    if args.top < sorted_counts.len() {
        println!("Top {} words:", args.top);
    } else {
        println!("Word frequency:");
    }

    // On prend seulement les N premiers
    for (word, count) in sorted_counts.into_iter().take(args.top) {
        println!("{}: {}", word, count);
    }

    Ok(())
}