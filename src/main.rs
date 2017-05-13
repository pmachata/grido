/*
 * Grido is a console game
 * Copyright (C) 2015, 2016 Petr Machata <pmachata@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate ncurses;
extern crate time;
extern crate rand;

use ncurses as nc;
use rand::Rng;

#[derive(Copy, Clone, Debug)]
enum Pen {
    None,
    Thin,
    Thik,
}

impl Pen {
    fn combine(p1: Pen, p2: Pen) -> Pen {
        match (p1, p2) {
            (Pen::None, p) => p,
            (p, Pen::None) => p,
            (Pen::Thik, _) |
            (_, Pen::Thik) => Pen::Thik,
            (Pen::Thin, Pen::Thin) => Pen::Thin,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Copy, Clone, Debug)]
struct FieldDrawing {
    up: Pen,
    right: Pen,
    down: Pen,
    left: Pen,
}

impl FieldDrawing {
    fn new_from(d: Direction, p: Pen) -> FieldDrawing {
        let mut up = Pen::None;
        let mut right = Pen::None;
        let mut down = Pen::None;
        let mut left = Pen::None;

        match d {
            Direction::Up    => up = p,
            Direction::Right => right = p,
            Direction::Down  => down = p,
            Direction::Left  => left = p,
        }

        FieldDrawing {up: up, right: right, down: down, left: left}
    }

    fn combine(&self, other: FieldDrawing) -> FieldDrawing {
        FieldDrawing {up:    Pen::combine(self.up, other.up),
                      right: Pen::combine(self.right, other.right),
                      down:  Pen::combine(self.down, other.down),
                      left:  Pen::combine(self.left, other.left)}
    }
}

#[derive(Copy, Clone, Debug)]
enum Field {
    None,
    Decoration(char),
    Drawing(FieldDrawing),
}

#[derive(Debug)]
struct Grid {
    w: i16,
    h: i16,

    grid: Vec<Field>,
}

impl Grid {
    fn new(w: i16, h: i16) -> Grid {
        assert!(w >= 0);
        assert!(h >= 0);

        let mut grid = Vec::new();
        for _ in 0.. (w + 1) * (h + 1) {
            grid.push(Field::None);
        }

        Grid {w:w as i16, h:h as i16, grid:grid}
    }

    fn field_idx(&self, x: i16, y: i16) -> usize {
        y as usize * (self.w + 1) as usize + x as usize
    }

    fn field_mut(&mut self, x: i16, y: i16) -> &mut Field {
        let idx = self.field_idx(x, y);
        &mut self.grid[idx]
    }

    fn paint(&mut self, x: i16, y: i16, d: Direction, p: Pen) {
        let f = self.field_mut(x, y);
        *f = match *f {
            Field::None |
            Field::Decoration(..)
                => Field::Drawing(FieldDrawing::new_from(d, p)),

            Field::Drawing(dw)
                => Field::Drawing(dw.combine(FieldDrawing::new_from(d, p))),
        }
    }

    fn clear(&mut self, x: i16, y: i16, w: i16, h: i16) {
        assert!(w >= 0);
        assert!(h >= 0);

        // Inner portion can be wiped simply.
        for xx in x .. x+w {
            for yy in y .. y+h {
                // Left or right edge.
                let ex0 = xx == x;
                let ex1 = xx == x+w-1;
                let ex = ex0 || ex1;

                // Upper or lower edge.
                let ey0 = yy == y;
                let ey1 = yy == y+h-1;
                let ey = ey0 || ey1;

                let f = self.field_mut(xx, yy);

                if !ex && !ey {
                    // Non-edge tile.
                    *f = Field::None;
                } else {
                    if ex {
                        if let Field::Drawing(ref mut dw) = *f {
                            if !ey {
                                dw.down = Pen::None;
                                dw.up = Pen::None;
                            }

                            if ex0 {
                                dw.right = Pen::None;
                            } else {
                                dw.left = Pen::None;
                            }
                        } else {
                            *f = Field::None;
                        }
                    }

                    if ey {
                        // Upper or lower edge.  We erase the
                        // horizontal and the inner (down or up) arm.
                        if let Field::Drawing(ref mut dw) = *f {
                            if !ex {
                                dw.left = Pen::None;
                                dw.right = Pen::None;
                            }

                            if ey0 {
                                dw.down = Pen::None;
                            } else {
                                dw.up = Pen::None;
                            }
                        } else {
                            *f = Field::None;
                        }
                    }
                }
            }
        }
    }

    fn paint_decoration(&mut self, x: i16, y: i16, s: &str) {
        let mut n = 0;
        for c in s.chars() {
            if c != '\0' {
                *self.field_mut(x+n, y) = Field::Decoration(c);
            }
            n += 1;
        }
    }

    fn paint_wall(&mut self, x0: i16, y0: i16, len: i16,
                  d: Direction, inclusive: bool, p: Pen) {
        assert!(len >= 0);

        let (d1, d2, dx, dy) = match d {
            Direction::Right => (Direction::Right, Direction::Left, 1, 0),
            Direction::Left => (Direction::Left, Direction::Right, -1, 0),
            Direction::Down => (Direction::Down, Direction::Up, 0, 1),
            Direction::Up => (Direction::Up, Direction::Down, 0, -1),
        };

        let x1 = x0 + len * dx;
        let y1 = y0 + len * dy;

        if inclusive {
            self.paint(x0, y0, d1, p);
        }
        for i in 1..len {
            let xx = x0 + i * dx;
            let yy = y0 + i * dy;
            self.paint(xx, yy, d2, p);
            self.paint(xx, yy, d1, p);
        }
        if inclusive {
            self.paint(x1, y1, d2, p);
        }
    }

    fn render_field_drawing(dw: FieldDrawing) -> &'static str {
        match (dw.up, dw.right, dw.down, dw.left) {
            (Pen::None, Pen::None, Pen::None, Pen::None) => " ",

            (Pen::None, Pen::None, Pen::None, Pen::Thin) => "╴",
            (Pen::None, Pen::None, Pen::Thin, Pen::None) => "╷",
            (Pen::None, Pen::None, Pen::Thin, Pen::Thin) => "┐",
            (Pen::None, Pen::Thin, Pen::None, Pen::None) => "╶",
            (Pen::None, Pen::Thin, Pen::None, Pen::Thin) => "─",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::None) => "┌",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::Thin) => "┬",
            (Pen::Thin, Pen::None, Pen::None, Pen::None) => "╵",
            (Pen::Thin, Pen::None, Pen::None, Pen::Thin) => "┘",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::None) => "│",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::Thin) => "┤",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::None) => "└",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::Thin) => "┴",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::None) => "├",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::Thin) => "┼",

            (Pen::None, Pen::None, Pen::None, Pen::Thik) => "╸",
            (Pen::None, Pen::None, Pen::Thik, Pen::None) => "╻",
            (Pen::None, Pen::None, Pen::Thik, Pen::Thik) => "┓",
            (Pen::None, Pen::Thik, Pen::None, Pen::None) => "╺",
            (Pen::None, Pen::Thik, Pen::None, Pen::Thik) => "━",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::None) => "┏",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::Thik) => "┳",
            (Pen::Thik, Pen::None, Pen::None, Pen::None) => "╹",
            (Pen::Thik, Pen::None, Pen::None, Pen::Thik) => "┛",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::None) => "┃",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::Thik) => "┫",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::None) => "┗",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::Thik) => "┻",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::None) => "┣",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::Thik) => "╋",

            (Pen::None, Pen::None, Pen::Thik, Pen::Thin) => "┒",
            (Pen::None, Pen::None, Pen::Thin, Pen::Thik) => "┑",
            (Pen::None, Pen::Thik, Pen::None, Pen::Thin) => "╼",
            (Pen::None, Pen::Thik, Pen::Thik, Pen::Thin) => "┲",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::None) => "┍",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::Thik) => "┯",
            (Pen::None, Pen::Thik, Pen::Thin, Pen::Thin) => "┮",
            (Pen::None, Pen::Thin, Pen::None, Pen::Thik) => "╾",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::None) => "┎",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::Thik) => "┱",
            (Pen::None, Pen::Thin, Pen::Thik, Pen::Thin) => "┰",
            (Pen::None, Pen::Thin, Pen::Thin, Pen::Thik) => "┭",
            (Pen::Thik, Pen::None, Pen::None, Pen::Thin) => "┚",
            (Pen::Thik, Pen::None, Pen::Thik, Pen::Thin) => "┨",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::None) => "╿",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::Thik) => "┩",
            (Pen::Thik, Pen::None, Pen::Thin, Pen::Thin) => "┦",
            (Pen::Thik, Pen::Thik, Pen::None, Pen::Thin) => "┺",
            (Pen::Thik, Pen::Thik, Pen::Thik, Pen::Thin) => "╊",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::None) => "┡",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::Thik) => "╇",
            (Pen::Thik, Pen::Thik, Pen::Thin, Pen::Thin) => "╄",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::None) => "┖",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::Thik) => "┹",
            (Pen::Thik, Pen::Thin, Pen::None, Pen::Thin) => "┸",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::None) => "┠",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::Thik) => "╉",
            (Pen::Thik, Pen::Thin, Pen::Thik, Pen::Thin) => "╂",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::None) => "┞",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::Thik) => "╃",
            (Pen::Thik, Pen::Thin, Pen::Thin, Pen::Thin) => "╀",
            (Pen::Thin, Pen::None, Pen::None, Pen::Thik) => "┙",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::None) => "╽",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::Thik) => "┪",
            (Pen::Thin, Pen::None, Pen::Thik, Pen::Thin) => "┧",
            (Pen::Thin, Pen::None, Pen::Thin, Pen::Thik) => "┥",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::None) => "┕",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::Thik) => "┷",
            (Pen::Thin, Pen::Thik, Pen::None, Pen::Thin) => "┶",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::None) => "┢",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::Thik) => "╈",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::Thin) => "┾",
            (Pen::Thin, Pen::Thik, Pen::Thik, Pen::Thin) => "╆",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::None) => "┝",
            (Pen::Thin, Pen::Thik, Pen::Thin, Pen::Thik) => "┿",
            (Pen::Thin, Pen::Thin, Pen::None, Pen::Thik) => "┵",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::None) => "┟",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::Thik) => "╅",
            (Pen::Thin, Pen::Thin, Pen::Thik, Pen::Thin) => "╁",
            (Pen::Thin, Pen::Thin, Pen::Thin, Pen::Thik) => "┽",
        }
    }

    fn render(&self, x0: i16, y0: i16) {
        for y in 0..self.h+1 {
            for x in 0..self.w+1 {
                match self.grid[self.field_idx(x, y)] {
                    Field::None => {
                    },

                    Field::Decoration(c) => {
                        if c != '\0' {
                            nc::mvprintw(y0 as i32 + y as i32,
                                         x0 as i32 + x as i32,
                                         &c.to_string());
                        }
                    },

                    Field::Drawing(dw) => {
                        nc::mvprintw(y0 as i32 + y as i32,
                                     x0 as i32 + x as i32,
                                     Grid::render_field_drawing(dw));
                    },
                };
            }
        }
    }
}


fn level(score: u32) -> u8 {
    let mut base: u32 = 0;
    let mut lvl: u8 = 0;
    while base < score {
        lvl += 1;
        base += lvl as u32 * 100;
    }
    lvl
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum LiquidType {
    Acid,
    Glue,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum TileType {
    Plain(u8),
    Permanent,
    Killer(u8),
    Picker,
    Centerpiece(u8),
    Whopper(u8),
    Flask(LiquidType),
    Spillage(LiquidType),
    Plus,
    Minus,
}

#[derive(PartialEq,Debug)]
enum ExplodeAction {
    Remove,
    Convert(TileType),
    Spill(LiquidType),
    Plus,
    Minus,
    Complex(Box<ExplodeAction>, Box<ExplodeAction>),
}

impl TileType {
    fn new_random(score: u32) -> TileType {
        let lvl = level(score);
        let mut rng = rand::thread_rng();
        loop {
            match rng.gen_range(0, 33) {
                0...20 => return TileType::Plain(0),

                21...23 => return TileType::Picker,

                24 if lvl >= 1
                    => return if rng.gen() { TileType::Minus }
                              else { TileType::Plus },

                25...26 if lvl >= 2
                    => return TileType::Plain(1 + rng.gen_range(0, lvl)),

                27 if lvl >= 3
                    => return TileType::Flask(if rng.gen() { LiquidType::Acid }
                                              else { LiquidType::Glue }),

                28 if lvl >= 4
                    => return TileType::Killer(1 + rng.gen_range(0, lvl / 8 + 1)),


                29...30 if lvl >= 5
                    => return TileType::Centerpiece(1 + rng.gen_range(0, lvl / 4 + 1)),

                31 if lvl >= 6
                    => return TileType::Whopper(1 + rng.gen_range(0, lvl / 4 + 1)),

                32 if lvl >= 8
                    => if rng.gen() { return TileType::Permanent },

                _ => {},
            }
        }
    }

    fn render(&self) -> &'static str {
        match *self {
            TileType::Permanent               => " ✖ ",
            TileType::Picker                  => "[ ]",
            TileType::Flask(LiquidType::Glue) => " ▿ ",
            TileType::Flask(LiquidType::Acid) => " ▴ ",
            TileType::Plus                    => " + ",
            TileType::Minus                   => " - ",

            TileType::Plain(n) => match n {
                0 => "   ",
                1 => " • ",
                2 => " •²",
                3 => " •³",
                4 => " •⁴",
                5 => " •⁵",
                6 => " •⁶",
                7 => " •⁷",
                8 => " •⁸",
                9 => " •⁹",
                _ => " •ⁿ",
            },

            TileType::Killer(n) => match n {
                0 => " ↯⁰",
                1 => " ↯ ",
                2 => " ↯²",
                3 => " ↯³",
                4 => " ↯⁴",
                5 => " ↯⁵",
                6 => " ↯⁶",
                7 => " ↯⁷",
                8 => " ↯⁸",
                9 => " ↯⁹",
                _ => " ↯ⁿ",
            },

            TileType::Centerpiece(n) => match n {
                0 => " ◉⁰",
                1 => " ◉ ",
                2 => " ◉²",
                3 => " ◉³",
                4 => " ◉⁴",
                5 => " ◉⁵",
                6 => " ◉⁶",
                7 => " ◉⁷",
                8 => " ◉⁸",
                9 => " ◉⁹",
                _ => " ◉ⁿ",
            },

            TileType::Whopper(n) => match n {
                0 => " ✱⁰",
                1 => " ✱ ",
                2 => " ✱²",
                3 => " ✱³",
                4 => " ✱⁴",
                5 => " ✱⁵",
                6 => " ✱⁶",
                7 => " ✱⁷",
                8 => " ✱⁸",
                9 => " ✱⁹",
                _ => " ✱ⁿ",
            },

            // Spills are formatted differently.
            TileType::Spillage(LiquidType::Glue) => "▿",
            TileType::Spillage(LiquidType::Acid) => "▴",
        }
    }

    fn drop(&self) -> Option<TileType> {
        match *self {
            TileType::Killer(_) => Some(TileType::Plain(0)),
            tt => Some(tt),
        }
    }

    fn explode(&self) -> ExplodeAction {
        use ExplodeAction::*;
        match *self {
            TileType::Plain(0) => Remove,
            TileType::Plain(n) => Convert(TileType::Plain(n - 1)),

            TileType::Centerpiece(1) => Remove,
            TileType::Centerpiece(n) => Convert(TileType::Centerpiece(n - 1)),

            TileType::Whopper(n) => Convert(TileType::Centerpiece(n)),

            TileType::Flask(liquid) => Spill(liquid),

            TileType::Plus => Complex(Box::new(Remove), Box::new(Plus)),
            TileType::Minus => Complex(Box::new(Remove), Box::new(Minus)),

            _ => Remove,
        }
    }

    fn is_plain(&self) -> bool {
        match *self {
            TileType::Plain(_) |
            TileType::Flask(..) |
            TileType::Plus |
            TileType::Minus => true,
            _ => false,
        }
    }

    fn is_solid(&self) -> bool {
        match *self {
            TileType::Spillage(_) => false,
            _ => true,
        }
    }

    fn explodes(&self, tt2: TileType) -> bool {
        match *self {
            // Plain tiles explode other plain tiles (not e.g.
            // Centerpieces).
            tt1 if tt1.is_plain() => tt2.is_plain(),

            // If Centerpieces is a centerpiece, it explodes other
            // plain tiles or Centerpieces.
            TileType::Centerpiece(_) => {
                if let TileType::Centerpiece(_) = tt2 {
                    true
                } else {
                    tt2.is_plain()
                }
            },

            // If Whopper is a centerpiece, it explodes other plain
            // tiles, Centerpieces and Whoppers.
            TileType::Whopper(_) => {
                match tt2 {
                    TileType::Centerpiece(_) |
                    TileType::Whopper(_) => true,
                    _ => tt2.is_plain(),
                }
            },

            // The rest doesn't explode.
            _ => false,
        }
    }

    fn collide(t1: TileType, t2: TileType) -> (Option<TileType>, Option<TileType>) {
        match (t1, t2) {
            // Liquids are never on the block.
            (_, TileType::Spillage(LiquidType::Acid)) => (None, None),
            (_, TileType::Spillage(LiquidType::Glue)) => (None, t1.drop()),

            (TileType::Picker, _) => (t2.drop(), None),
            (_, TileType::Picker) => (None, t1.drop()),

            (TileType::Killer(1), _) => (None, None),
            (TileType::Killer(n), _) => (Some(TileType::Killer(n - 1)), None),

            (_, TileType::Killer(1)) => (None, None),
            (_, TileType::Killer(n)) => (None, Some(TileType::Killer(n - 1))),

            (m, n) => (Some(m), Some(n)),
        }
    }

    fn collides(t1: TileType, t2: TileType) -> bool {
        match (t1, t2) {
            _ => true,
        }
    }

    fn explode_shape(&self) -> &'static [(i16, i16)] {
        match *self {
            TileType::Whopper(_) => {
                static SHAPE:[(i16, i16); 25] = [(-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2),
                                                 (-2, -1), (-1, -1), (0, -1), (1, -1), (2, -1),
                                                 (-2,  0), (-1,  0), (0,  0), (1,  0), (2,  0),
                                                 (-2,  1), (-1,  1), (0,  1), (1,  1), (2,  1),
                                                 (-2,  2), (-1,  2), (0,  2), (1,  2), (2,  2)];
                &SHAPE
            },

            _ => {
                static SHAPE:[(i16, i16); 9] = [(-1, -1), (0, -1), (1, -1),
                                                (-1,  0), (0,  0), (1,  0),
                                                (-1,  1), (0,  1), (1,  1)];
                &SHAPE
            },
        }
    }

    fn bonus(&self) -> u32 {
        match *self {
            TileType::Plain(n) => n as u32 + 1,
            TileType::Centerpiece(n) => 10 * n as u32,
            TileType::Whopper(_) => 30,
            _ => 1,
        }
    }
}

#[derive(Debug)]
struct Block {
    x: i16,
    y: i16,
    tiles: Vec<(i16, i16, TileType)>,
}

impl Block {
    fn new() -> Block {
        Block::new_at(0, 0)
    }

    fn new_at(x: i16, y: i16) -> Block {
        Block {x: x, y: y, tiles:vec![]}
    }

    fn new_from_shape(shape: &[(i16, i16)], score: u32) -> Block {
        let mut rtiles = Vec::new();
        for &(dx, dy) in shape {
            rtiles.push((dx, dy, TileType::new_random(score)));
        }
        Block {x:0, y:0, tiles:rtiles}
    }

    fn new_random(score: u32) -> Block {
        fn shape_1x1() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 1] = [(0, 0)];
            &SHAPE
        }

        fn shape_8() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 2] = [( 0, -1), ( 0,  1)];
            &SHAPE
        }

        fn shape_d() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 2] = [(-1, -1), ( 0, 0)];
            &SHAPE
        }

        fn shape_l() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 3] = [(-1, 0), (0,  0), ( 0, -1)];
            &SHAPE
        }

        fn shape_1x2() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 2]
                = [( 0, -1), ( 0,  0)];
            &SHAPE
        }

        fn shape_1x3() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 3]
                = [( 0, -1), ( 0,  0), (0,  1)];
            &SHAPE
        }

        fn shape_castle() -> &'static [(i16, i16)] {
            static SHAPE:[(i16, i16); 3] = [( 0, -1), (-1,  0), ( 1,  0)];
            &SHAPE
        }

        return match rand::random::<u8>() % 7 {
            0 => Block::new_from_shape(shape_1x1(), score),
            1 => Block::new_from_shape(shape_1x2(), score),
            2 => Block::new_from_shape(shape_1x3(), score),
            3 => Block::new_from_shape(shape_8(), score),
            4 => Block::new_from_shape(shape_d(), score),
            5 => Block::new_from_shape(shape_l(), score),
            6 => Block::new_from_shape(shape_castle(), score),
            _ => unreachable!(),
        }
    }

    fn new_border(w: i16, h: i16) -> Block {
        assert!(w >= 0);
        assert!(h >= 0);

        let mut tiles = Vec::new();
        for x in 0..w-1 {
            tiles.push((x, 0, TileType::Permanent));
            tiles.push((x+1, h-1, TileType::Permanent));
        }
        for y in 0..h-1 {
            tiles.push((0, y+1, TileType::Permanent));
            tiles.push((w-1, y, TileType::Permanent));
        }
        Block {x:0, y:0, tiles:tiles}
    }

    fn paint_tile(x0: i16, y0: i16, w: i16, h: i16, grid: &mut Grid,
                  have_up: bool, have_right: bool,
                  have_down: bool, have_left: bool,
                  pen1: Pen, pen2: Pen) {
        let x1 = x0 + w;
        let y1 = y0 + h;
        let pen = |b: bool| { if b { pen1 } else { pen2 } };

        for &(x, y, len, d, n) in [(x0, y0, w, Direction::Right, have_up),
                                   (x1, y0, h, Direction::Down, have_right),
                                   (x0, y1, w, Direction::Right, have_down),
                                   (x0, y0, h, Direction::Down, have_left)].iter() {
            grid.paint_wall(x, y, len, d, !n, pen(n));
        }
    }

    fn paint1(&self, x: i16, y: i16, tt: TileType, grid: &mut Grid) {
        let up = self.at(x, y-1);
        let right = self.at(x+1, y);
        let down = self.at(x, y+1);
        let left = self.at(x-1, y);

        // A tile is 5x3, but the walls are shared, so we place
        // them to dx*4, dy*2.
        let tx = 4 * x;
        let ty = 2 * y;
        grid.clear(tx, ty, 5, 3);

        if tt.is_solid() {
            fn is_solid_neighbor(n: Option<TileType>) -> bool {
                if let Some(tt) = n {
                    tt.is_solid()
                } else {
                    false
                }
            }

            Block::paint_tile(tx, ty, 4, 2, grid,
                              is_solid_neighbor(up), is_solid_neighbor(right),
                              is_solid_neighbor(down), is_solid_neighbor(left),
                              Pen::Thin, Pen::Thik);
            grid.paint_decoration(tx + 1, ty + 1, tt.render());
        } else {
            let c = tt.render();
            grid.paint_decoration(tx, ty+0, &format!(" {} {} ", c, c));
            grid.paint_decoration(tx, ty+1, &format!("{} {} {}", c, c, c));
            grid.paint_decoration(tx, ty+2, &format!(" {} {} ", c, c));
        }
    }

    fn paint(&self, grid: &mut Grid) {
        let &Block {x:x0, y:y0, ref tiles} = self;

        for &(dx, dy, tt) in tiles {
            if ! tt.is_solid() {
                self.paint1(x0 + dx, y0 + dy, tt, grid);
            }
        }

        for &(dx, dy, tt) in tiles {
            if tt.is_solid() {
                self.paint1(x0 + dx, y0 + dy, tt, grid);
            }
        }
    }

    fn at(&self, x: i16, y: i16) -> Option<TileType> {
        let &Block {x:x0, y:y0, ref tiles} = self;
        for &(dx, dy, tt) in tiles {
            if x == x0+dx && y == y0+dy {
                return Some(tt)
            }
        }
        None
    }

    fn turned(&self) -> Block {
        let &Block {x:x0, y:y0, ref tiles} = self;

        let mut rtiles = Vec::with_capacity(tiles.len());
        for &(dx, dy, tt) in tiles {
            rtiles.push((dy, -dx, tt));
        }
        Block {x:x0, y:y0, tiles:rtiles}
    }

    fn moved(&self, dx: i16, dy: i16) -> Block {
        let &Block {x:x0, y:y0, ref tiles} = self;
        let mut rtiles = Vec::with_capacity(tiles.len());
        for &tile in tiles {
            rtiles.push(tile);
        }
        Block {x:x0+dx, y:y0+dy, tiles:rtiles}
    }

    fn moved_to(&self, x: i16, y: i16) -> Block {
        self.moved(x - self.x, y - self.y)
    }

    fn collide(blk1: Block, blk2: &Block) -> (Block, Block) {
        let Block {x:x1, y:y1, tiles:tiles1} = blk1;
        let mut rtiles1 = Vec::new();

        let &Block {x:x2, y:y2, ..} = blk2;
        let mut rtiles2 = Vec::new();
        for &(dx, dy, tt2) in &blk2.tiles {
            rtiles2.push((dx, dy, tt2));
        }

        for (dx1, dy1, tt1) in tiles1 {
            let xx1 = x1 + dx1;
            let yy1 = y1 + dy1;
            if let Some(tt2) = blk2.at(xx1, yy1) {
                if TileType::collides(tt1, tt2) {
                    let (nt1, nt2) = TileType::collide(tt1, tt2);
                    if let Some(ntt1) = nt1 {
                        rtiles1.push((dx1, dy1, ntt1));
                    }
                    rtiles2.retain(|&(dx2, dy2, _): &(i16, i16, TileType)|
                                   x2 + dx2 != xx1 || y2 + dy2 != yy1);
                    if let Some(ntt2) = nt2 {
                        rtiles2.push((xx1 - blk2.x, yy1 - blk2.y, ntt2));
                    }
                } else {
                    rtiles1.push((dx1, dy1, tt1));
                }
            } else {
                rtiles1.push((dx1, dy1, tt1));
            }
        }

        (Block {x:blk1.x, y:blk1.y, tiles:rtiles1},
         Block {x:blk2.x, y:blk2.y, tiles:rtiles2})
    }

    fn intersects(&self, blk2: &Block) -> bool {
        let &Block {x:x1, y:y1, ref tiles} = self;
        for &(dx1, dy1, _) in tiles {
            let xx1 = x1 + dx1;
            let yy1 = y1 + dy1;
            if let Some(_) = blk2.at(xx1, yy1) {
                return true;
            }
        }
        false
    }

    fn drop(&self, dest: &mut Block, bd: &Block) -> bool {
        if self.intersects(bd) || self.intersects(dest) {
            return false;
        }

        let &Block {x:x1, y:y1, ref tiles} = self;
        let &mut Block {x:x2, y:y2, tiles:ref mut dtiles} = dest;
        let ddx = x1 - x2;
        let ddy = y1 - y2;
        for &(dx1, dy1, tt1) in tiles {
            if let Some(ntt) = tt1.drop() {
                dtiles.push((dx1 + ddx, dy1 + ddy, ntt));
            }
        }

        true
    }

    fn collides_with(&self, with: &Block) -> bool {
        let &Block {x:x0, y:y0, ref tiles} = self;
        for &(dx, dy, tt1) in tiles {
            if let Some(tt2) = with.at(x0+dx, y0+dy) {
                if TileType::collides(tt1, tt2) {
                    return true;
                }
            }
        }
        false
    }

    fn spill(x: i16, y: i16, spills: &mut Vec<(i16, i16, LiquidType)>, liquid: LiquidType) {
        for &(dx, dy) in &[(0, 0), (0, 1), (1, 0), (0, -1), (-1, 0)] {
            spills.push((x+dx, y+dy, liquid));
        }
    }

    fn explode(&mut self) -> (Vec<(i16, i16, TileType)>, u32, i32) {
        let mut killlist = Vec::new();

        {
            'next: for &(xx, yy, tt) in &self.tiles {
                let mut sublist = Vec::new();
                for &(dx, dy) in tt.explode_shape() {
                    let x2 = self.x + xx + dx;
                    let y2 = self.y + yy + dy;
                    match self.at(x2, y2) {
                        None => continue 'next,
                        Some(tt2) => if tt.explodes(tt2) {
                            sublist.push((x2, y2));
                        } else {
                            continue 'next
                        },
                    }
                }
                for i in sublist {
                    killlist.push(i);
                }
            }
        }

        let mut exploded = Vec::new();

        fn handle_xp_action(xa: ExplodeAction, xx: i16, yy: i16,
                            spills: &mut Vec<(i16, i16, LiquidType)>,
                            rtiles: &mut Vec<(i16, i16, TileType)>) -> i32 {
            match xa {
                ExplodeAction::Remove => {
                    0
                },
                ExplodeAction::Convert(tt2) => {
                    rtiles.push((xx, yy, tt2));
                    0
                },
                ExplodeAction::Spill(liquid) => {
                    Block::spill(xx, yy, spills, liquid);
                    0
                },
                ExplodeAction::Plus => 1,
                ExplodeAction::Minus => -1,
                ExplodeAction::Complex(a, b) => {
                    handle_xp_action(*a, xx, yy, spills, rtiles)
                        + handle_xp_action(*b, xx, yy, spills, rtiles)
                },
            }
        };

        let mut hits = 0;
        let mut dmult = 0;
        {
            let mut rtiles = Vec::new();
            let mut spills = Vec::new();
            'next2: for &(xx, yy, tt) in &self.tiles {
                for &(x2, y2) in &killlist {
                    if self.x + xx == x2 && self.y + yy == y2 {
                        exploded.push((xx, yy, tt));
                        dmult += handle_xp_action(tt.explode(), xx, yy,
                                                  &mut spills, &mut rtiles);
                        hits += tt.bonus();
                        continue 'next2;
                    }
                }
                rtiles.push((xx, yy, tt));
            }

            self.tiles = rtiles;

            for (xx, yy, liquid) in spills {
                if ! self.at(xx, yy).is_some() {
                    self.tiles.push((xx, yy, TileType::Spillage(liquid)));
                }
            }
        }

        if exploded.len() > 12 {
            dmult += (exploded.len() as i32 - 9) / 9;
        }

        (exploded, hits, dmult)
    }
}

#[derive(Debug)]
struct Particle {
    x: f32,
    y: f32,
    face: String,
    start: time::SteadyTime,
    ttl: u32,
}

impl Particle {
    fn new(x: f32, y: f32, face: String, ttl: u32) -> Particle {
        Particle {x: x, y: y, face: face, start: time::SteadyTime::now(), ttl: ttl}
    }

    fn paint(&self, grid: &mut Grid) {
        grid.paint_decoration(self.x as i16, self.y as i16, &self.face);
    }

    fn dead(&self) -> bool {
        let ttl = time::Duration::milliseconds(self.ttl as i64);
        time::SteadyTime::now() - self.start > ttl
    }
}

fn play() {
    let (pgw, pgh) = (16 as i16, 12 as i16);
    let mut score = 0;
    let mut blk = Block::new_random(score).moved_to(2, 2);
    let mut next = Block::new_random(score).moved_to(1, 1);
    let bd = Block::new_border(pgw, pgh);
    let mut pg = Block::new();
    let mut particles: Vec<Particle> = Vec::new();

    let mut last_drop_time = time::SteadyTime::now();

    let mut multiplier: u32 = 1;
    let mut last_mult_time = last_drop_time;

    loop {
        let mut drop = false;
        let mut mult_drop = false;

        particles.retain(|p: &Particle| !p.dead());
        {
            let mut grid = Grid::new(4 * pgw, 2 * pgh);
            for xx in 0..grid.w {
                for yy in 0..grid.h {
                    if xx % 3 == yy % 3 {
                        grid.paint_decoration(xx, yy, ".");
                    }
                }
            }

            grid.clear(5, 3, 12, 6);
            for xx in 0..3 {
                grid.paint_wall(6 + 4 * xx, 2, 6, Direction::Down,
                                true, Pen::Thin);
            }
            for yy in 0..3 {
                grid.paint_wall(4, 3 + 2 * yy, 12, Direction::Right,
                                true, Pen::Thin);
            }

            pg.paint(&mut grid);
            bd.paint(&mut grid);
            blk.paint(&mut grid);

            let mut gridlet = Grid::new(12, 6);
            next.paint(&mut gridlet);

            fn paint_gauge(start: &time::SteadyTime, limit: i64) -> (String, bool) {
                let dtime = time::SteadyTime::now() - *start;
                let mut remaining = limit - dtime.num_milliseconds();
                if remaining < 0 {
                    remaining = 0;
                }

                // 96 is 12 * 8: 12 characters times 8 different widths of
                // unicode block.
                let frac = (96.0 * (limit as f32 - remaining as f32) / limit as f32) as i32;
                let mut timebar = "◂".to_string();
                for _ in 0 .. (frac / 8) {
                    timebar.push_str("█");
                }
                if remaining > 0 {
                    timebar.push_str(match frac % 8 {
                        0 => " ",
                        1 => "▏",
                        2 => "▎",
                        3 => "▍",
                        4 => "▌",
                        5 => "▋",
                        6 => "▊",
                        7 => "▉",
                        _ => "",
                    });
                }
                for _ in (frac / 8) .. 11 {
                    timebar.push_str(" ");
                }
                timebar.push_str("▸");

                (timebar, remaining == 0)
            }

            let (timebar, over) = paint_gauge(&last_drop_time, 15000);
            if over {
                drop = true;
            }

            let (mult_timebar, mult_over) = paint_gauge(&last_mult_time, 60000);
            if mult_over && multiplier != 1 {
                mult_drop = true;
            }

            for p in &particles {
                p.paint(&mut grid);
            }

            nc::erase();
            grid.render(0, 0);
            gridlet.render(grid.w + 1, 0);
            nc::mvprintw(gridlet.h as i32 + 1, grid.w as i32 + 1, &timebar);
            nc::mvprintw(gridlet.h as i32 + 2, grid.w as i32 + 1,
                         &format!("Score: {}", score));
            nc::mvprintw(gridlet.h as i32 + 3, grid.w as i32 + 1,
                         &format!("Level: {}", level(score)));

            nc::mvprintw(gridlet.h as i32 + 5, grid.w as i32 + 1, &mult_timebar);
            nc::mvprintw(gridlet.h as i32 + 6, grid.w as i32 + 1,
                         &format!("Multi: x{}", multiplier));
            nc::refresh();
        }

        fn block_collides(block: &Block, bd: &Block, pg: &Block) -> bool {
            block.collides_with(&bd) || block.collides_with(&pg)
        }

        fn try_move(moved: Block, blk: Block, bd: &Block, pg: &mut Block) -> Block {
            if moved.intersects(bd) {
                blk
            } else if moved.collides_with(pg) {
                let (moved2, pg2) = Block::collide(moved, pg);
                if moved2.collides_with(&pg2) {
                    blk
                } else {
                    *pg = pg2;
                    moved2
                }
            } else {
                moved
            }
        };

        nc::timeout(20);
        match nc::getch() {
            nc::KEY_LEFT => blk = try_move(blk.moved(-1, 0), blk, &bd, &mut pg),
            nc::KEY_RIGHT => blk = try_move(blk.moved(1, 0), blk, &bd, &mut pg),
            nc::KEY_UP => blk = try_move(blk.moved(0, -1), blk, &bd, &mut pg),
            nc::KEY_DOWN => blk = try_move(blk.moved(0, 1), blk, &bd, &mut pg),
            nc::KEY_BACKSPACE => {
                let moved = next.moved_to(blk.x, blk.y);
                if !block_collides(&moved, &bd, &pg) {
                    next = blk.moved_to(1, 1);
                    blk = moved;
                }
            },

            n => match n as u8 as char {
                '\t' => blk = try_move(blk.turned(), blk, &bd, &mut pg),
                '\r' => {
                    let grace = time::Duration::milliseconds(500);
                    if time::SteadyTime::now() - last_drop_time > grace {
                        drop = true;
                    }
                },
                /*
                ' ' => blk = Block::new_random(score).moved_to(2, 2),
                '+' => score += 500,
                '*' => multiplier += 1,
                */
                'q' => break,
                'p' => {
                    let pause_start = time::SteadyTime::now();
                    nc::erase();
                    nc::mvprintw(pgh as i32, 2 * pgw as i32 - 3, "Pause.");
                    nc::timeout(-1);
                    nc::getch();
                    let now = time::SteadyTime::now();
                    last_drop_time = last_drop_time + (now - pause_start);
                    last_mult_time = last_mult_time + (now - pause_start);
                },
                _ => {
                    /*
                    nc::endwin();
                    println!("{}", n);
                    return
                     */
                },
            }
        }

        if blk.tiles.is_empty() || drop {
            if blk.drop(&mut pg, &bd) {
                last_drop_time = time::SteadyTime::now();
                let (_, hits, dmult) = pg.explode();
                let bonus = hits * multiplier;
                score += bonus;

                if dmult != 0 {
                    if dmult < 0 {
                        if -dmult as u32 >= multiplier {
                            multiplier = 0
                        } else {
                            multiplier -= -dmult as u32;
                        }
                    } else {
                        multiplier += dmult as u32;
                    }

                    last_mult_time = time::SteadyTime::now();
                }

                if bonus > 0 {
                    particles.push(Particle::new(4. * blk.x as f32, 2. * blk.y as f32,
                                                 format!("{}", bonus), 5000));
                }

                if dmult > 0 {
                    particles.push(Particle::new(4. * blk.x as f32, 1. + 2. * blk.y as f32,
                                                 format!("+x{}", dmult), 5000));
                } else if dmult < 0 {
                    particles.push(Particle::new(4. * blk.x as f32, 1. + 2. * blk.y as f32,
                                                 format!("-x{}", -dmult), 5000));
                }

                blk = next.moved(1, 1);
                next = Block::new_random(score).moved_to(1, 1);
                if block_collides(&blk, &bd, &pg) {
                    break;
                }
            }
        }

        if mult_drop {
            multiplier = if multiplier > 1 { multiplier - 1 }
            		 else { multiplier + 1 };
            last_mult_time = time::SteadyTime::now();
        } else if multiplier == 1 {
            last_mult_time = time::SteadyTime::now();
        }
    }
}

#[derive(Copy, Clone)]
enum MenuAction {
    Play,
    Help,
    Quit,
}

fn logo() {
    nc::mvprintw(2, 1, "╶─╼━━━━━━━━━━━╾─╴");
    nc::mvprintw(3, 1, "╶╼ G R I D - O ╾╴");
    nc::mvprintw(4, 1, "╶─╼━━━━━━━━━━━╾─╴");
}

fn menu() -> MenuAction {
    let mut pos: i32 = 0;

    let items = [("Play", MenuAction::Play),
                 ("Help", MenuAction::Help),
                 ("Quit", MenuAction::Quit)];

    loop {
        nc::erase();
        logo();
        for i in 0..items.len() {
            if i == pos as usize {
                nc::mvprintw(i as i32 + 6, 1, "➤");
            }
            let &(text, _) = &items[i];
            nc::mvprintw(i as i32 + 6, 3, text);
        }

        nc::timeout(-1);
        match nc::getch() {
            nc::KEY_UP => pos -= 1,
            nc::KEY_DOWN => pos += 1,
            n => match n as u8 as char {
                '\r' => {
                    let &(_, action) = &items[pos as usize];
                    return action;
                },
                'p' => return MenuAction::Play,
                'h' => return MenuAction::Help,
                'q' => return MenuAction::Quit,
                _ => {},
            },
        }

        if pos < 0 {
            pos = 0;
        }
        if pos >= items.len() as i32 {
            pos = (items.len() - 1) as i32;
        }
    }
}

fn help() {
    nc::erase();
    logo();

    let mut grid = Grid::new(80, 24);

    let mut y = 3;
    let mut x = 1;
    for &(tts, descr)
        in &[(&vec![TileType::Plain(0)],
              "Plain tiles.  When organized\ninto a 3x3, explode and\n\
               disappear.  1 point."),
             (&vec![TileType::Plain(1), TileType::Plain(3)],
              "Shield tiles.  When exploded,\ndecrease the number, eventually\n\
               change to plain.  n+1 points."),
             (&vec![TileType::Centerpiece(1), TileType::Centerpiece(3)],
              "Centerpiece.  Only explode\nwhen 3x3 has a centerpiece\n\
               in the center.  10*n points."),
             (&vec![TileType::Whopper(1), TileType::Whopper(3)],
              "Whopper.  Like centerpiece\nbut only explodes 5x5.  When\n\
               exploded, changes to c-piece\nwith the same number.\n\
               30 points."),
             (&vec![TileType::Picker],
              "Picker.  Doesn't explode.\nAllows picking other tiles."),
             (&vec![TileType::Killer(1), TileType::Killer(3)],
              "Killer.  Kills tiles that\nit touches.  On drop,\nchanges to plain."),
             (&vec![TileType::Permanent],
              "Permanent.\nNever explodes.\nKill them!"),
             (&vec![TileType::Plus, TileType::Minus],
              "Plus, Minus.  When exploded,\nchange the multiplier.\n\
               1 point."),
             (&vec![TileType::Flask(LiquidType::Glue),
                    TileType::Flask(LiquidType::Acid)],
              "Flask with Glue and Acid.\nSpill contents around\n\
               when exploded.  1 point.")] {
        {
            let mut blk = Block::new_at(x, y);
            for i in 0..tts.len() {
                blk.tiles.push((-(tts.len() as i16) + i as i16 + 1, 0, tts[i]));
            }
            blk.paint(&mut grid);

            let mut dy = 0;
            for k in descr.split("\n") {
                grid.paint_decoration(4 * (x + 1) + 2, 2 * y + dy, k);
                dy += 1
            }
        }
        y += 2;
        if y > 9 {
            y = 1;
            x += 10;
        }
    }

    grid.render(0, 0);

    nc::getch();

    nc::erase();
    logo();
    nc::mvprintw(6, 1,  "⬅⬆⬇➡  Arrows: move current block around the playground.");
    nc::mvprintw(7, 1,  "   ↲  Enter: drop the block.");
    nc::mvprintw(8, 1,  "   ⇰  Tab: rotate the block.");
    nc::mvprintw(9, 1,  "   ⇦  Backspace: swap current block with the next block.");
    nc::mvprintw(12, 1, "   p  Pause game.");
    nc::mvprintw(13, 1, "   q  Quit game--go back to the menu.");

    nc::getch();
}

fn main() {
    nc::setlocale(nc::LcCategory::all, "");

    nc::initscr();
    nc::keypad(unsafe {nc::stdscr}, true);
    nc::nonl();
    nc::cbreak();
    nc::raw();
    nc::noecho();
    nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    loop {
        match menu() {
            MenuAction::Play => play(),
            MenuAction::Help => help(),
            MenuAction::Quit => break,
        }
    }

    nc::endwin();
}
