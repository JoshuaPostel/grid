extern crate grid;

extern crate ansi_term;
extern crate rand;
extern crate ndarray;
extern crate itertools;
extern crate termion;
extern crate rodio;
#[macro_use] extern crate prettytable;

use std::io::Read;
use std::io::BufReader;
use std::collections::HashMap;
use termion::raw::IntoRawMode;
use prettytable::Table;

//TODO remove module_inception
use grid::grid::grid::Grid;
use grid::tetris::tile::{Tile, SQUARE_OUTLINE};
use grid::tetris::tetrad::{Tetrad, Queue};

trait Update {

    fn add_tetrad(&mut self, tetrad: &Tetrad);
    fn remove_tetrad(&mut self, tetrad: &Tetrad);            

    fn add_tile(&mut self, tile: Tile);            
    fn remove_tile(&mut self, tile: Tile);            

    fn valid_tile(&self, tile: Tile) -> bool;            
    fn full_rows(&self) -> Vec<usize>;
    fn clear_rows(&mut self, full_rows: Vec<usize>);
}

impl Update for Grid<Tile> {


    fn add_tetrad(&mut self, tetrad: &Tetrad) {
        for tile in tetrad.tiles.iter() {
            self.add_tile(*tile);
        }
    }

    fn remove_tetrad(&mut self, tetrad: &Tetrad) {
        for tile in tetrad.tiles.iter() {
            self.remove_tile(*tile);
        }
    }

    fn add_tile(&mut self, tile: Tile) {
        self.grid[[tile.row, tile.column]] = tile;
    }

    fn remove_tile(&mut self, tile: Tile) {
        let default_tile = Tile { row: tile.row, column: tile.column, ..Default::default() };
        self.grid[[tile.row, tile.column]] = default_tile;
    }

    fn valid_tile(&self, tile: Tile) -> bool {
        let row_good = tile.row < self.height;
        let column_good = tile.column < self.width;
        let mut location_ocupied = false;
        if row_good && column_good {
            location_ocupied = self.grid[[tile.row, tile.column]].empty;
        }
        row_good && column_good && location_ocupied
    }

    fn full_rows(&self) -> Vec<usize> {
        let mut full_rows = Vec::new();
        for row in self.grid.genrows() {
            let full_row = row.into_iter().all(|tile| tile.empty == false);
            if full_row {
                full_rows.push(row[0].row)
            }
        }
        full_rows
    }

    fn clear_rows(&mut self, full_rows: Vec<usize>) {
    
        for full_row in full_rows {
            let _ = self.grid.row_mut(full_row).map_mut(std::mem::take);
            for row_index in (0..full_row).rev() {
                let mut bottom_row = self.grid.row_mut(row_index).map_mut(std::mem::take);
                for tile in bottom_row.iter_mut() {
                    tile.row += 1;
                }
                self.grid.row_mut(row_index + 1).assign(&bottom_row);
            }
        }
    }
}

trait Move {

    fn move_tetrad(&mut self, grid: &Grid<Tile>, tetrad_mover: Box<dyn Fn(&mut Tetrad)>) -> bool;
}

impl Move for Tetrad {

    fn move_tetrad(&mut self, grid: &Grid<Tile>, tetrad_mover: Box<dyn Fn(&mut Tetrad)>) -> bool {
        let mut new_tetrad = self.clone();
        tetrad_mover(&mut new_tetrad);

        let valid_move = new_tetrad.tiles
            .iter()
            .all(|tile| grid.valid_tile(*tile));
        if valid_move {    
            tetrad_mover(self);
        }
        valid_move
    }
}

struct Tetris {
    grid: Grid<Tile>,
    active_tetrad: Tetrad,
    tetrad_shadow: Tetrad,
    queue: Queue,
    held_tetrad: Option<String>,
    score: usize,
    lines: usize,
    level: usize,
}

impl Tetris {

    fn update_level(&mut self) {
        self.level = (self.lines / 10) + 1;
    }

    fn move_active_tetrad(&mut self, tetrad_mover: Box<dyn Fn(&mut Tetrad)>) -> bool { 
        let mut tetrad = self.active_tetrad.clone();
        self.grid.remove_tetrad(&self.active_tetrad);
        let was_moved = tetrad.move_tetrad(&self.grid, tetrad_mover);
        self.grid.add_tetrad(&tetrad);
        self.active_tetrad = tetrad;
        self.update_shadow();
        was_moved
    }

    fn get_shadow(&self) -> Tetrad {
        let mut shadow = self.active_tetrad.clone();
        while shadow.tiles.iter().all(|tile| self.grid.valid_tile(*tile)) {
            shadow.tiles.iter_mut().for_each(|tile| tile.row += 1);
        }
        for tile in shadow.tiles.iter_mut() {
            //better way to avoid panic?
            if tile.row > 0 {
                tile.row -= 1;
            }
            tile.utf8 = SQUARE_OUTLINE;
        }
        shadow
    }

    fn update_shadow(&mut self) {
        self.grid.remove_tetrad(&self.tetrad_shadow);
        let shadow = self.get_shadow();
        self.grid.add_tetrad(&shadow);
        self.tetrad_shadow = shadow;
        self.grid.add_tetrad(&self.active_tetrad);
    } 

    fn hard_drop(&mut self) { 
        self.grid.remove_tetrad(&self.active_tetrad);
        let color = self.active_tetrad.tiles[0].color;
        let utf8 = self.active_tetrad.tiles[0].utf8;
        let from_row = &self.active_tetrad.tiles[0].row;
        let to_row = &self.tetrad_shadow.tiles[0].row;
        let rows_dropped = to_row - from_row;
        self.score += rows_dropped * self.level * 2;
        self.active_tetrad.tiles = self.tetrad_shadow.tiles;
        for tile in self.active_tetrad.tiles.iter_mut() {
            tile.color = color;
            tile.utf8 = utf8;
        }
    }

    //TODO
    //better python like function wrapping?
    fn move_left(&mut self) { 
        fn move_tetrad_left(tetrad: &mut Tetrad) {
            //TODO shouldnt need a specific check for move left 
            let legal = tetrad.tiles.iter().all(|x| x.column > 0);
            if legal {
                for tile in tetrad.tiles.iter_mut() {
                    tile.column -= 1;
                }
                tetrad.center.1 -= 1.0;
            }
        }
        let _was_moved = self.move_active_tetrad(Box::new(move_tetrad_left));
    }

    fn move_right(&mut self) { 
        fn move_tetrad_right(tetrad: &mut Tetrad) {
            tetrad.tiles.iter_mut().for_each(|tile| tile.column += 1);
            tetrad.center.1 += 1.0;
        }
        let _was_moved = self.move_active_tetrad(Box::new(move_tetrad_right));
    }
    
    fn move_down(&mut self, score: usize) { 
        fn move_tetrad_down(tetrad: &mut Tetrad) {
            tetrad.tiles.iter_mut().for_each(|tile| tile.row += 1);
            tetrad.center.0 += 1.0;
        }
        let was_moved = self.move_active_tetrad(Box::new(move_tetrad_down));
        if was_moved {
            self.score += score;
        }
    }

    fn rotate_tetrad(tetrad: &mut Tetrad, rotation_matrix: [[f32; 2]; 2]) {
        for tile in tetrad.tiles.iter_mut() {
            let row = tile.row as f32;
            let center_row = tetrad.center.0 as f32;
            let column = tile.column as f32;
            let center_column = tetrad.center.1 as f32;
            let normalized = ndarray::arr2(
                &[[row - center_row],[column - center_column]]);
            let rotation_matrix = ndarray::arr2(&rotation_matrix);
            let rotated = rotation_matrix.dot(&normalized);
            let new_row = rotated[[0,0]] + center_row;
            let new_column = rotated[[1,0]] + center_column;
            tile.row = new_row as usize;
            tile.column = new_column as usize;
        }
    }

    fn rotate_left(&mut self) {
        
        fn rotate_tetrad_left(tetrad: &mut Tetrad) {
            let rotation_matrix: [[f32; 2]; 2] = [[0.,-1.],[1.,0.]];
            Tetris::rotate_tetrad(tetrad, rotation_matrix)
        }

        let _was_moved = self.move_active_tetrad(Box::new(rotate_tetrad_left));
    }

    fn rotate_right(&mut self) {
        
        fn rotate_tetrad_right(tetrad: &mut Tetrad) {
            let rotation_matrix: [[f32; 2]; 2] = [[0.,1.],[-1.,0.]];
            Tetris::rotate_tetrad(tetrad, rotation_matrix)
        }

        let _was_moved = self.move_active_tetrad(Box::new(rotate_tetrad_right));
    }


    //TODO remove mut?
    //better way than double reverse()?
    fn display(&mut self) {
        let mut display_queue = String::from("\nnext:\n");
        self.queue.tetrads.reverse();
        for tetrad in &self.queue.tetrads[..6] {
            display_queue.push_str(&tetrad.render);
            display_queue.push_str("\n");
        }
        self.queue.tetrads.reverse();

        let mut stats = String::from("\nscore:\n");
        stats.push_str(&self.score.to_string());
        stats.push_str("\n\nlines:\n");
        stats.push_str(&self.lines.to_string());
        stats.push_str("\n\nlevel:\n");
        stats.push_str(&self.level.to_string());

        let mut held = String::from("\nheld:\n");
        held.push_str(&self.render_held_tetrad());

        let mut held_and_stats = table!([held], [stats]);
        held_and_stats.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        let mut display_table = Table::new();
        display_table.add_row(
            row![held_and_stats,
                self.grid.display_string(),
                display_queue]);

        let display_string = display_table.to_string().replace("\n","\n\r");
        println!("{}[2J", 27 as char);
        println!("{}", display_string);
    }

    fn render_held_tetrad(&self) -> String {
        match &self.held_tetrad {
            Some(name) => Tetrad::new_by_name(name).render,
            None => "        \n\n\n".to_string()
        }
    }

    //TODO needs an "if legal" check
    fn hold(&mut self) {
        let active_tetrad_name = self.active_tetrad.name.clone();
        self.grid.remove_tetrad(&self.active_tetrad);
        match &self.held_tetrad {
            Some(name) => {
                self.active_tetrad = Tetrad::new_by_name(&name);
                self.update_shadow();
            },
            None => {
                self.active_tetrad = self.queue.next_tetrad();
                self.update_shadow();
            }
        }
        self.held_tetrad = Some(active_tetrad_name);
    }
}

fn vecs_match<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
    let matches = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
    matches == a.len() && matches == b.len()
}

fn on_new_tetrad(tetris: &mut Tetris, score_table: &HashMap<usize, usize>) -> bool {
    
    let mut game_live = true;
    tetris.active_tetrad.tiles.iter_mut().for_each(|tile| tile.empty = false);
    tetris.grid.add_tetrad(&tetris.active_tetrad);

    for tile in tetris.active_tetrad.tiles.iter_mut() {
        tile.empty = false;
    }

    let full_rows = tetris.grid.full_rows();
    let n_full_rows = full_rows.len();
    if n_full_rows > 0 {
        tetris.grid.clear_rows(full_rows);
        tetris.lines += n_full_rows;
        tetris.update_level();
        tetris.score += score_table.get(&n_full_rows).unwrap() * tetris.level;
    }

    tetris.active_tetrad = tetris.queue.next_tetrad();
    tetris.tetrad_shadow = tetris.get_shadow();
    tetris.grid.add_tetrad(&tetris.tetrad_shadow);

    let valid_move = tetris.active_tetrad.tiles
        .iter()
        .all(|tile| tetris.grid.valid_tile(*tile));
    if valid_move {    
        tetris.grid.add_tetrad(&tetris.active_tetrad);
    } else {
        game_live = false;
    }
    game_live
}

fn main() {

    let tetris_text = 
" _____    _        _   
|_   _|__| |_ _ __(_)___
  | |/ _ \\ __| '__| / __|
  | |  __/ |_| |  | \\__ \\
  |_|\\___|\\__|_|  |_|___/

";
    let controls_text = 
"left:          J  ⇦ 

right:         L  ⇨

down:          K  ⇩

hard drop:     I  ⇧  SPACE

rotate left:   D

rotate right:  F

hold:          S

quit:          Q
";
    let mut greeting = Table::new();
    greeting.add_row(row![tetris_text]);
    greeting.add_row(row!["   Press ENTER to begin"]);
    greeting.add_row(row![controls_text]);
    greeting.add_row(row![" Add tetris.mp3 for music"]);
    //TODO better way for frist screen clear?
    println!("{}", greeting.to_string());
    println!("{}[2J", 27 as char);
    println!("{}", greeting.to_string());



    //TODO move out of main: https://crates.io/crates/phf
    let mut score_table: HashMap<usize, usize> = HashMap::new();
    score_table.insert(1, 100);
    score_table.insert(2, 300);
    score_table.insert(3, 500);
    score_table.insert(4, 1000);

    let width: usize = 10;
    let height: usize = 24;
    let mut tiles: Vec<Tile> = Vec::new();
    for x in 0..width {
    	for y in 0..height {
        	tiles.push(Tile::new(x, y))
		}
    }
    let g = Grid::new(width, height, tiles);

    let mut queue = Queue::new();
    let mut tetris = Tetris { grid: g, 
        active_tetrad: queue.next_tetrad(),
        tetrad_shadow: Tetrad::new_l(), //placeholder
        queue: queue,
        held_tetrad: None,
        score: 0,
        lines: 0,
        level: 1}; 

    tetris.update_shadow();
    tetris.grid.add_tetrad(&tetris.active_tetrad);

    let _stdout = std::io::stdout();
    let _stdout = _stdout.lock().into_raw_mode().unwrap();
    let mut input = termion::async_stdin().bytes();

    let device = rodio::default_output_device().unwrap();
    let sink = rodio::Sink::new(&device);
    sink.set_volume(0.1);

    let mut can_hold = true;
    let mut game_live = true;

    loop {
        match input.next() {
            Some(Ok(13)) => break, //enter key
            Some(Ok(b'q')) => {
                game_live = false;
                break
            } 
            _ => continue,
        }
    }

    while game_live {

        if sink.empty() {
            match std::fs::File::open("tetris.mp3") {
                Err(_) => (),
                Ok(file) => sink.append(rodio::Decoder::new(BufReader::new(file)).unwrap()),
            };
        }

        let position_before = tetris.active_tetrad.get_position();
        tetris.move_down(0);
        let position_after = tetris.active_tetrad.get_position();

        if vecs_match(&position_before, &position_after) {

            game_live = on_new_tetrad(&mut tetris, &score_table);
            if !game_live { break }
            can_hold = true
        }

        let advance_rate = 1000 - tetris.level * 100;
        let mut next_drop = std::time::Duration::from_millis(advance_rate as u64);
        let last_drop = std::time::Instant::now();

        tetris.display();
        loop {
            let mut hard_dropped = false;
            let time_elapsed = last_drop.elapsed();
            if time_elapsed >= next_drop {
                break;
            }
            match input.next() {
                None => continue,
                Some(Ok(68)) | Some(Ok(b'j')) => tetris.move_left(), //left arrow
                Some(Ok(66)) | Some(Ok(b'k')) => tetris.move_down(tetris.level), //down arrow
                Some(Ok(67)) | Some(Ok(b'l')) => tetris.move_right(), //right arrow
                Some(Ok(b'd')) => tetris.rotate_left(),
                Some(Ok(b'f')) => tetris.rotate_right(),
                Some(Ok(b's')) => {
                    if can_hold {
                        tetris.hold();
                        can_hold = false;
                    }
                },
                //up arrow
                Some(Ok(65)) | Some(Ok(b'i')) | Some(Ok(b' ')) => {
                    tetris.hard_drop();
                    hard_dropped = true;
                    },
                Some(Ok(b'q')) => {
                    game_live = false;
                    break;
                },
                _ => break
            }
            if hard_dropped {
                break;
            }

            tetris.display();
            next_drop -= time_elapsed;
        }
    }
    println!("GAME OVER\r\n");
}
