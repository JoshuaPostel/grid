extern crate ansi_term;
extern crate rand;
extern crate ndarray;
extern crate itertools;
extern crate termion;


use crate::grid::rgb::RGB;
use crate::grid::grid::Depict;

pub const SQUARE: [u8; 4] = [0xE2, 0x96, 0xA0, 0x20];
pub const SQUARE_OUTLINE: [u8; 4] = [0xE2, 0x96, 0xA1, 0x20];
pub const OUTLINED_SQUARE: [u8; 4] = [0xE2, 0x96, 0xA3, 0x20];


#[derive(Copy, Clone, Debug)]
pub struct Tile {
    pub empty: bool,
    pub color: RGB,
    pub utf8: [u8; 4],
    pub row: usize,
    pub column: usize,
}

impl Default for Tile {
    fn default() -> Tile {
        Tile {
            empty: true,
            color: RGB { r: 47, g: 79, b: 79},
            utf8: SQUARE,
            row: 0,
            column: 0,
        }
    }
}

impl Tile {

    pub fn new(row: usize, column: usize) -> Tile {
        Tile { row, column, ..Default::default() }
    }
}

impl Depict for Tile {

    fn color(&self) -> RGB {
        self.color
    }

    fn utf8(&self) -> [u8; 4] {
        self.utf8
    }
}

//impl Tile {
//
//    fn advance(&mut self) {
//        self.row += 1;
//    }
//}
