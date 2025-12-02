use clap::Parser;
use rand::Rng; // Nécessaire pour .random()
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs;
use std::thread;
use std::time::Duration;
use std::process; // Pour exit(1)

// ==========================================
// CONFIGURATION & STRUCTURES
// ==========================================

#[derive(Parser, Debug)]
#[command(name = "hexpath", version, about = "Find min/max cost paths in hexadecimal grid")]
struct Args {
    /// Map file (hex values, space separated)
    #[arg(required_unless_present = "generate")]
    file: Option<String>,

    /// Generate random map (e.g., 8x4, 10x10)
    #[arg(long)]
    generate: Option<String>,

    /// Save generated map to file
    #[arg(long)]
    output: Option<String>,

    /// Show colored map
    #[arg(long)]
    visualize: bool,

    /// Show both min and max paths
    #[arg(long)]
    both: bool,

    /// Animate pathfinding
    #[arg(long)]
    animate: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct State {
    cost: u32,
    x: usize,
    y: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost) // Min-heap
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Grid {
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl Grid {
    fn new(width: usize, height: usize, cells: Vec<u8>) -> Self {
        Self { width, height, cells }
    }

    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn get_xy(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }
    
    fn get_val(&self, x: usize, y: usize) -> u8 {
        self.cells[self.get_index(x, y)]
    }
}

// ==========================================
// MAIN LOGIC
// ==========================================

fn main() {
    let args = Args::parse();

    // 1. GENERATION DE MAP
    // Note: on utilise &args.generate pour ne pas consommer 'args'
    if let Some(dim_str) = &args.generate {
        let parts: Vec<&str> = dim_str.split('x').collect();
        if parts.len() != 2 {
            eprintln!("Invalid format. Use WxH (e.g., 10x10)");
            process::exit(1); // CORRECTION : Exit code 1
        }
        let w: usize = parts[0].parse().unwrap_or(10);
        let h: usize = parts[1].parse().unwrap_or(10);

        println!("Generating {}x{} hexadecimal grid...", w, h);
        
        // CORRECTION : Rand 0.9+ syntaxe
        let mut rng = rand::rng();
        let mut cells = vec![0u8; w * h];
        
        for i in 0..cells.len() {
            cells[i] = rng.random(); 
        }

        // Force Start (00) and End (FF)
        cells[0] = 0x00;
        cells[w * h - 1] = 0xFF;

        // Affichage brut
        print_grid_values(&cells, w);

        // Sauvegarde
        if let Some(out_file) = &args.output {
            let mut content = String::new();
            for (i, val) in cells.iter().enumerate() {
                content.push_str(&format!("{:02X}", val));
                if (i + 1) % w == 0 { content.push('\n'); } else { content.push(' '); }
            }
            if let Err(e) = fs::write(out_file, content) {
                eprintln!("Error writing file: {}", e);
                process::exit(1); // CORRECTION : Exit code 1
            } else {
                println!("Map saved to: {}", out_file);
            }
        }

        if !args.visualize && !args.both && !args.animate {
            return;
        }
        
        process_grid(Grid::new(w, h, cells), &args);
        return;
    }

    // 2. LECTURE DE FICHIER
    if let Some(file_path) = &args.file {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                let mut cells = Vec::new();
                let mut width = 0;
                let mut height = 0;

                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    let row_vals: Vec<u8> = line.split_whitespace()
                        .map(|s| u8::from_str_radix(s, 16).unwrap_or(0))
                        .collect();
                    if width == 0 { width = row_vals.len(); }
                    cells.extend(row_vals);
                    height += 1;
                }
                
                if width == 0 || height == 0 {
                    eprintln!("Empty or invalid map file.");
                    process::exit(1); // CORRECTION : Exit code 1
                }

                if args.generate.is_none() {
                    println!("Analyzing hexadecimal grid...");
                    println!("Grid size: {}x{}", width, height);
                    println!("Start: (0,0) = 0x{:02X}", cells[0]);
                    println!("End: ({},{}) = 0x{:02X}", width - 1, height - 1, cells[cells.len() - 1]);
                }

                process_grid(Grid::new(width, height, cells), &args);
            }
            Err(e) => {
                eprintln!("Could not read file: {}", e);
                process::exit(1); // CORRECTION : Exit code 1
            }
        }
    }
}

fn process_grid(grid: Grid, args: &Args) {
    if args.visualize {
        println!("\nHEXADECIMAL GRID (rainbow gradient):");
        println!("========================================");
        print_colored_grid(&grid, &[]);
    }

    if args.animate {
        println!("\nSearching for minimum cost path...");
        let (path, _cost) = find_path(&grid, false, true);
        if let Some(p) = path {
             println!("\nStep {}: Path found!", p.len());
             print_colored_grid(&grid, &p);
        }
        return; 
    }

    // Calcul du chemin MIN
    println!("\nMINIMUM COST PATH:");
    println!("==================");
    let (min_path, min_cost) = find_path(&grid, false, false);
    
    if let Some(path) = &min_path {
        print_path_stats(path, min_cost, &grid);
        if args.visualize {
             println!("\nMINIMUM COST PATH (shown in WHITE):");
             println!("===================================");
             print_colored_grid(&grid, path);
        }
    } else {
        println!("No path found!");
    }

    // Calcul du chemin MAX
    if args.both {
        println!("\nMAXIMUM COST PATH:");
        println!("==================");
        let (max_path, _max_cost_inverted) = find_path(&grid, true, false);
        
        if let Some(path) = &max_path {
            print_path_stats(path, 0, &grid); 
            
            if args.visualize {
                println!("\nMAXIMUM COST PATH (shown in WHITE):");
                print_colored_grid(&grid, path);
            }
        }
    }
}

// ==========================================
// ALGORITHME DIJKSTRA
// ==========================================

fn find_path(grid: &Grid, maximize: bool, animate: bool) -> (Option<Vec<usize>>, u32) {
    let start_idx = 0;
    let end_idx = grid.cells.len() - 1;

    let mut dist = vec![u32::MAX; grid.cells.len()];
    let mut heap = BinaryHeap::new();
    let mut parents: HashMap<usize, usize> = HashMap::new();

    dist[start_idx] = 0;
    heap.push(State { cost: 0, x: 0, y: 0 });

    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
    
    let mut steps_count = 0;

    while let Some(State { cost, x, y }) = heap.pop() {
        let current_idx = grid.get_index(x, y);

        if current_idx == end_idx {
            // Reconstruct path
            let mut path = Vec::new();
            let mut curr = end_idx;
            path.push(curr);
            while let Some(&p) = parents.get(&curr) {
                curr = p;
                path.push(curr);
            }
            path.reverse();
            return (Some(path), cost);
        }

        if cost > dist[current_idx] {
            continue;
        }

        if animate {
            if steps_count % 5 == 0 { 
                print!("\x1B[2J\x1B[1;1H"); 
                println!("Searching for minimum cost path...\n");
                println!("Step {}: Exploring ({},{}) - cost: {}", steps_count, x, y, cost);
                print_anim_grid(grid, x, y, &parents);
                thread::sleep(Duration::from_millis(20));
            }
            steps_count += 1;
        }

        for (dx, dy) in directions {
            let new_x = x as isize + dx;
            let new_y = y as isize + dy;

            if new_x >= 0 && new_x < grid.width as isize && new_y >= 0 && new_y < grid.height as isize {
                let nx = new_x as usize;
                let ny = new_y as usize;
                let next_idx = grid.get_index(nx, ny);
                
                let cell_val = grid.get_val(nx, ny) as u32;
                let move_cost = if maximize { 255 - cell_val } else { cell_val };
                
                let next_cost = cost + move_cost;

                if next_cost < dist[next_idx] {
                    dist[next_idx] = next_cost;
                    parents.insert(next_idx, current_idx);
                    heap.push(State { cost: next_cost, x: nx, y: ny });
                }
            }
        }
    }

    (None, 0)
}

// ==========================================
// AFFICHAGE & TOOLS
// ==========================================

fn print_grid_values(cells: &[u8], width: usize) {
    println!("Generated map:");
    for (i, val) in cells.iter().enumerate() {
        print!("{:02X} ", val);
        if (i + 1) % width == 0 { println!(); }
    }
    println!();
}

fn print_path_stats(path: &[usize], _algo_cost: u32, grid: &Grid) {
    let mut total_real: u32 = 0;
    
    // Calcul du vrai coût pour affichage
    for (i, &idx) in path.iter().enumerate() {
        let (x, y) = grid.get_xy(idx);
        let val = grid.get_val(x, y);
        if i > 0 { 
             total_real += val as u32;
        }
    }
    
    println!("Total cost: 0x{:X} ({} decimal)", total_real, total_real);
    println!("Path length: {} steps", path.len()); 
    
    if path.len() < 30 { 
        println!("Path:");
        let coords: Vec<String> = path.iter().map(|&idx| {
            let (x, y) = grid.get_xy(idx);
            format!("({},{})", x, y)
        }).collect();
        println!("{}", coords.join("->"));

        println!("\nStep-by-step costs:");
        let mut running_cost = 0;
        for (i, &idx) in path.iter().enumerate() {
            let (x, y) = grid.get_xy(idx);
            let val = grid.get_val(x, y);
            if i == 0 {
                println!("Start 0x{:02X} (0,0)", val);
            } else {
                running_cost += val as u32;
                println!("-> 0x{:02X} ({},{}) +{}", val, x, y, running_cost);
            }
        }
        println!("Total: 0x{:X} ({})", running_cost, running_cost);
    }
}

fn print_colored_grid(grid: &Grid, path: &[usize]) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let idx = grid.get_index(x, y);
            let val = grid.cells[idx];
            let is_path = path.contains(&idx);

            if is_path {
                print!("\x1b[48;2;255;255;255m\x1b[38;2;0;0;0m {:02X} \x1b[0m", val);
            } else {
                let (r, g, b) = hex_to_rgb(val);
                print!("\x1b[38;2;{};{};{}m{:02X} \x1b[0m", r, g, b, val);
            }
        }
        println!();
    }
}

fn print_anim_grid(grid: &Grid, cur_x: usize, cur_y: usize, parents: &HashMap<usize, usize>) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let idx = grid.get_index(x, y);
            if x == cur_x && y == cur_y {
                print!("[*]");
            } else if parents.contains_key(&idx) || idx == 0 {
                print!("[✓]");
            } else {
                print!("[ ]");
            }
        }
        println!();
    }
}

fn hex_to_rgb(val: u8) -> (u8, u8, u8) {
    if val < 128 {
        let ratio = val as f32 / 128.0;
        let r = ((1.0 - ratio) * 255.0) as u8;
        let g = (ratio * 255.0) as u8;
        (r + 50, g + 50, 0)
    } else {
        let ratio = (val - 128) as f32 / 127.0;
        let g = ((1.0 - ratio) * 255.0) as u8;
        let b = (ratio * 255.0) as u8;
        (0, g + 50, b + 50)
    }
}